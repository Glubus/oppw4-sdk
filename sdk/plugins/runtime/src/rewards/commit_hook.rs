mod format;

use std::{
    ffi::{c_char, c_void},
    mem, panic,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::{OwnedHostApi, PluginResult};
use serde::{Deserialize, Serialize};

use crate::{
    config::RewardProbeConfig,
    runtime::{
        core::{
            rank::RankValue,
            rewards::{RewardCommitEvent, RewardState},
        },
        signals,
    },
};

use super::apply;

pub(super) const PLUGIN_ID: &str = "sdk_runtime";

const REWARD_COMMIT_SIGNATURE: Signature = Signature::new(
    "reward_commit_14132a670",
    &[
        0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18,
        0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xec, 0x30, 0x8b, 0xea,
    ],
    &[
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ],
);

const OVERWRITE_LEN: usize = 15;

type RewardCommitFn = extern "system" fn(*mut u64, u32, u32, i32, i32, i32) -> *mut u64;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static MUTATION_APPLICATORS_INSTALLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct RewardCommitOutcome {
    event: RewardCommitEvent,
}

#[derive(Debug, Serialize)]
struct RewardCommitSnapshot {
    call: usize,
    reward_out: usize,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
    slots: [u64; apply::REWARD_SLOT_COUNT],
}

pub(crate) fn install(host: OwnedHostApi, config: RewardProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "reward_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "reward_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(REWARD_COMMIT_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump(reward_commit_detour as *const () as usize)?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let _ = HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "reward_probe installed site=0x{site:x} trampoline=0x{:x} max_logs={}",
                    hook.trampoline, config.max_logs
                ),
            );
        }
        Err(error) => {
            let _ = host
                .log()
                .write(PLUGIN_ID, format!("reward_probe install failed: {error}"));
        }
    }
}

pub(crate) fn install_mutation_applicators(host: OwnedHostApi) {
    if let Err(error) = register_mutation_applicators(host.clone()) {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("reward mutation applicator install failed: {error}"),
        );
    }
}

pub(crate) fn register_mutation_applicators(host: OwnedHostApi) -> PluginResult<()> {
    if MUTATION_APPLICATORS_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }
    let result = unsafe {
        host.signals().subscribe_bytes(
            signals::REWARD_BERRY_SET_TOTAL,
            std::ptr::null_mut(),
            apply_reward_berry_total_mutation,
        )
    };
    if result.is_err() {
        MUTATION_APPLICATORS_INSTALLED.store(false, Ordering::Release);
    }
    result
}

extern "system" fn reward_commit_detour(
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
) -> *mut u64 {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return reward_out;
    }

    let original: RewardCommitFn = unsafe { mem::transmute(original) };
    let result = original(
        reward_out,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
    );
    let outcome = panic::catch_unwind(|| reward_commit_event_from_memory(reward_out, rank_or_mode))
        .ok()
        .flatten()
        .map(|event| RewardCommitOutcome { event });

    let _ = panic::catch_unwind(|| {
        apply::clear_pending_berry_total();
        log_reward(
            reward_out,
            reward_param,
            mission_or_reward,
            rank_or_mode,
            bonus_a,
            bonus_b,
            outcome.as_ref(),
        );
    });
    apply::apply_pending_berry_total(reward_out, HOST.get());

    result
}

pub(crate) fn request_berry_total(total: u64) {
    apply::request_berry_total(total);
}

unsafe extern "system" fn apply_reward_berry_total_mutation(
    _subscriber_context: *mut c_void,
    _signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    if payload.is_null() && payload_len != 0 {
        return -2;
    }
    let bytes = if payload_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(payload, payload_len) }
    };
    let Ok(envelope) = serde_json::from_slice::<MutationSignalEnvelope>(bytes) else {
        return -26;
    };
    let Some(total) = envelope.payload.total else {
        return -23;
    };
    request_berry_total(total);
    0
}

#[derive(Debug, Deserialize)]
struct MutationSignalEnvelope {
    payload: BerrySetTotalPayload,
}

#[derive(Debug, Deserialize)]
struct BerrySetTotalPayload {
    total: Option<u64>,
}

fn log_reward(
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
    outcome: Option<&RewardCommitOutcome>,
) {
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    if reward_out.is_null() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "reward_probe call={} reward_out=null param2={} param3={} param4={} param5={} param6={}",
                index + 1,
                reward_param,
                mission_or_reward,
                rank_or_mode,
                bonus_a,
                bonus_b
            ),
        );
        return;
    }

    let snapshot = format::snapshot(
        index + 1,
        reward_out,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
    );
    let _ = host.log().write(PLUGIN_ID, format::reward_log(&snapshot));
    signals::emit_json(host, signals::REWARD_COMMIT, &snapshot);
    if let Some(outcome) = outcome {
        let _ = host.log().write(
            PLUGIN_ID,
            format::reward_event_log(index + 1, &outcome.event),
        );
        signals::emit_json(
            host,
            signals::REWARD_EVENT,
            &format::reward_event_payload(&outcome.event),
        );
    }
}

fn reward_commit_event_from_memory(
    reward_out: *mut u64,
    rank_or_mode: i32,
) -> Option<RewardCommitEvent> {
    if reward_out.is_null() {
        return None;
    }

    let slots = unsafe {
        std::slice::from_raw_parts(reward_out.cast_const(), apply::REWARD_SLOT_COUNT)
            .try_into()
            .unwrap_or([0; apply::REWARD_SLOT_COUNT])
    };
    Some(reward_commit_event_from_slots(rank_or_mode, &slots))
}

fn reward_commit_event_from_slots(
    rank_or_mode: i32,
    slots: &[u64; apply::REWARD_SLOT_COUNT],
) -> RewardCommitEvent {
    let rank = RankValue::from_slot(rank_or_mode.clamp(0, u8::MAX as i32) as u8);
    RewardCommitEvent::new(
        rank,
        RewardState::new().with_berry(slots[apply::BERRY_TOTAL_SLOT]),
    )
}

#[cfg(test)]
mod tests {
    use crate::runtime::core::rewards::RewardState;

    use super::*;

    #[test]
    fn reward_commit_event_uses_rank_and_berry_slot() {
        let slots = [0, 0, 0, 0, 0, 0, 321, 0];
        let event = reward_commit_event_from_slots(5, &slots);

        assert_eq!(event.rank.debug_alias(), "S+");
        assert_eq!(event.rewards, RewardState::new().with_berry(321));
    }

    #[test]
    fn berry_mutation_signal_sets_pending_total() {
        let _lock = apply::pending_test_lock();
        let mut slots = [0_u64; apply::REWARD_SLOT_COUNT];
        slots[apply::BERRY_TOTAL_SLOT] = 321;
        apply::clear_pending_berry_total();
        let payload = serde_json::json!({
            "schema": "sdk.host.mutation.v1",
            "key": "sdk.runtime.rewards.berry.set_total",
            "source_mod": "test_mod",
            "payload": { "total": 642 }
        })
        .to_string();

        let code = unsafe {
            apply_reward_berry_total_mutation(
                std::ptr::null_mut(),
                std::ptr::null(),
                payload.as_ptr(),
                payload.len(),
            )
        };

        assert_eq!(code, 0);
        apply::apply_pending_berry_total(slots.as_mut_ptr(), None);
        assert_eq!(slots[apply::BERRY_TOTAL_SLOT], 642);
    }
}
