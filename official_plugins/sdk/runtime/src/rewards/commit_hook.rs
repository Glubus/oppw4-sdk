mod format;

use std::{
    mem, panic,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::{
    config::RewardProbeConfig,
    runtime::{
        core::{
            events::RuntimeMutation,
            rank::RankValue,
            rewards::{RewardCommitEvent, RewardMutation, RewardState},
        },
        signals,
    },
};

const PLUGIN_ID: &str = "sdk_runtime";

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
pub(super) const REWARD_SLOT_COUNT: usize = 8;
const BERRY_TOTAL_SLOT: usize = 6;

type RewardCommitFn = extern "system" fn(*mut u64, u32, u32, i32, i32, i32) -> *mut u64;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct RewardCommitOutcome {
    event: RewardCommitEvent,
    mutations: Vec<RuntimeMutation>,
    error_count: usize,
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
    slots: [u64; REWARD_SLOT_COUNT],
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
    let outcome = panic::catch_unwind(|| apply_core_reward_mutations(reward_out, rank_or_mode))
        .ok()
        .flatten();

    let _ = panic::catch_unwind(|| {
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

    result
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
        if !outcome.mutations.is_empty() || outcome.error_count != 0 {
            let _ = host
                .log()
                .write(PLUGIN_ID, format::reward_mutations_log(&outcome.mutations));
        }
        signals::emit_json(
            host,
            signals::REWARD_EVENT,
            &format::reward_event_payload(&outcome.event, &outcome.mutations),
        );
    }
}

fn apply_core_reward_mutations(
    reward_out: *mut u64,
    rank_or_mode: i32,
) -> Option<RewardCommitOutcome> {
    if reward_out.is_null() {
        return None;
    }

    let before = unsafe { reward_out.add(BERRY_TOTAL_SLOT).read() };
    let event = reward_commit_event_from_memory(reward_out, rank_or_mode);
    let report = super::dispatch_reward_event(&event);
    let after = apply_reward_mutations_to_berry(
        event.rewards.berry.map(|berry| berry.amount),
        &report.mutations,
    );

    if let Some(after) = after {
        if after != before {
            unsafe {
                reward_out.add(BERRY_TOTAL_SLOT).write(after);
            }
        }
        log_berry_mutation(before, after, report.mutations.len(), report.errors.len());
    }

    Some(RewardCommitOutcome {
        event,
        mutations: report.mutations,
        error_count: report.errors.len(),
    })
}

fn reward_commit_event_from_memory(reward_out: *mut u64, rank_or_mode: i32) -> RewardCommitEvent {
    let slots = unsafe {
        std::slice::from_raw_parts(reward_out.cast_const(), REWARD_SLOT_COUNT)
            .try_into()
            .unwrap_or([0; REWARD_SLOT_COUNT])
    };
    reward_commit_event_from_slots(rank_or_mode, &slots)
}

fn reward_commit_event_from_slots(
    rank_or_mode: i32,
    slots: &[u64; REWARD_SLOT_COUNT],
) -> RewardCommitEvent {
    let rank = RankValue::from_slot(rank_or_mode.clamp(0, u8::MAX as i32) as u8);
    RewardCommitEvent::new(rank, RewardState::new().with_berry(slots[BERRY_TOTAL_SLOT]))
}

fn apply_reward_mutations_to_berry(
    amount: Option<u64>,
    mutations: &[RuntimeMutation],
) -> Option<u64> {
    let mut amount = amount?;

    for mutation in mutations {
        let RuntimeMutation::Reward(RewardMutation::MultiplyBerry(factor)) = mutation else {
            continue;
        };
        amount = scale_berry(amount, *factor);
    }

    Some(amount)
}

fn scale_berry(amount: u64, factor: f64) -> u64 {
    if !factor.is_finite() || factor < 0.0 {
        return amount;
    }
    ((amount as f64) * factor)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64
}

fn log_berry_mutation(before: u64, after: u64, mutation_count: usize, error_count: usize) {
    if mutation_count == 0 && error_count == 0 {
        return;
    }
    let Some(host) = HOST.get() else {
        return;
    };
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "reward_apply berry_before={before} berry_after={after} mutations={mutation_count} errors={error_count}"
        ),
    );
}

#[cfg(test)]
mod tests {
    use crate::runtime::core::{
        events::RuntimeMutation,
        rewards::{RewardMutation, RewardState},
    };

    use super::*;

    #[test]
    fn multiply_berry_mutation_doubles_known_amount() {
        let amount = apply_reward_mutations_to_berry(
            Some(150),
            &[RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0))],
        );

        assert_eq!(amount, Some(300));
    }

    #[test]
    fn multiply_berry_preserves_absent_amount() {
        let amount = apply_reward_mutations_to_berry(
            None,
            &[RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0))],
        );

        assert_eq!(amount, None);
    }

    #[test]
    fn no_mutation_preserves_berry_amount() {
        let amount = apply_reward_mutations_to_berry(Some(150), &[]);

        assert_eq!(amount, Some(150));
    }

    #[test]
    fn invalid_multiply_berry_factor_preserves_amount() {
        let amount = apply_reward_mutations_to_berry(
            Some(150),
            &[RuntimeMutation::Reward(RewardMutation::MultiplyBerry(
                f64::NAN,
            ))],
        );

        assert_eq!(amount, Some(150));
    }

    #[test]
    fn multiply_berry_saturates_on_overflow() {
        let amount = apply_reward_mutations_to_berry(
            Some(u64::MAX - 1),
            &[RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0))],
        );

        assert_eq!(amount, Some(u64::MAX));
    }

    #[test]
    fn reward_commit_event_uses_rank_and_berry_slot() {
        let slots = [0, 0, 0, 0, 0, 0, 321, 0];
        let event = reward_commit_event_from_slots(5, &slots);

        assert_eq!(event.rank.debug_alias(), "S+");
        assert_eq!(event.rewards, RewardState::new().with_berry(321));
    }

    #[test]
    fn reward_commit_dispatches_bus_mutations_to_berry_slot() {
        let _guard = crate::runtime::core::live_bus::test_lock();
        crate::rewards::reset_reward_handlers_for_tests();
        crate::rewards::register_reward_handler("double_berry", |_| {
            Ok(vec![RuntimeMutation::Reward(
                RewardMutation::MultiplyBerry(2.0),
            )])
        });
        let mut slots = [0, 0, 0, 0, 0, 0, 200, 0];

        let outcome = apply_core_reward_mutations(slots.as_mut_ptr(), 5).expect("outcome");

        assert_eq!(slots[BERRY_TOTAL_SLOT], 400);
        assert_eq!(outcome.mutations.len(), 1);
        crate::rewards::reset_reward_handlers_for_tests();
    }
}
