mod format;

use std::{
    mem, panic,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
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
            rank::RankValue,
            rewards::{RewardCommitEvent, RewardState},
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
const BERRY_BALANCE_CAP: u64 = 999_999_999;
const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_OWNER_OFFSET: usize = 0x18;
const SAVE_PTR_OFFSET: usize = 0x10;
const SAVE_BERRY_BALANCE_OFFSET: usize = 0x14;

type RewardCommitFn = extern "system" fn(*mut u64, u32, u32, i32, i32, i32) -> *mut u64;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static PENDING_BERRY_TOTAL_SET: AtomicBool = AtomicBool::new(false);
static PENDING_BERRY_TOTAL: AtomicU64 = AtomicU64::new(0);

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
    let outcome = panic::catch_unwind(|| reward_commit_event_from_memory(reward_out, rank_or_mode))
        .ok()
        .flatten()
        .map(|event| RewardCommitOutcome { event });

    let _ = panic::catch_unwind(|| {
        clear_pending_berry_total();
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
    apply_pending_berry_total(reward_out);

    result
}

pub(crate) fn request_berry_total(total: u64) {
    PENDING_BERRY_TOTAL.store(total, Ordering::Relaxed);
    PENDING_BERRY_TOTAL_SET.store(true, Ordering::Release);
}

fn clear_pending_berry_total() {
    PENDING_BERRY_TOTAL_SET.store(false, Ordering::Release);
}

fn take_pending_berry_total() -> Option<u64> {
    PENDING_BERRY_TOTAL_SET
        .swap(false, Ordering::AcqRel)
        .then(|| PENDING_BERRY_TOTAL.load(Ordering::Relaxed))
}

fn apply_pending_berry_total(reward_out: *mut u64) {
    if reward_out.is_null() {
        clear_pending_berry_total();
        return;
    }
    let Some(total) = take_pending_berry_total() else {
        return;
    };
    let Some(adjustment) = (unsafe { apply_reward_buffer_berry_total(reward_out, total) }) else {
        return;
    };
    apply_save_berry_balance_delta(adjustment.delta);
}

unsafe fn apply_reward_buffer_berry_total(
    reward_out: *mut u64,
    total: u64,
) -> Option<BerryBalanceAdjustment> {
    let previous_total = unsafe { *reward_out.add(BERRY_TOTAL_SLOT) };
    let delta = i128::from(total) - i128::from(previous_total);
    unsafe {
        *reward_out.add(BERRY_TOTAL_SLOT) = total;
    }
    Some(BerryBalanceAdjustment {
        previous_total,
        total,
        delta,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BerryBalanceAdjustment {
    previous_total: u64,
    total: u64,
    delta: i128,
}

fn apply_save_berry_balance_delta(delta: i128) {
    if delta == 0 {
        return;
    }
    let Some(host) = HOST.get() else {
        return;
    };
    match patch_save_berry_balance(host, delta) {
        Ok(patch) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "reward_berry_balance_patch save=0x{:x} old={} new={} delta={}",
                    patch.save, patch.previous_balance, patch.balance, delta
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("reward_berry_balance_patch failed delta={delta}: {error}"),
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SaveBerryBalancePatch {
    save: usize,
    previous_balance: u64,
    balance: u64,
}

fn patch_save_berry_balance(
    host: &OwnedHostApi,
    delta: i128,
) -> Result<SaveBerryBalancePatch, String> {
    let save = read_save_pointer(host)?;
    let balance_address = save + SAVE_BERRY_BALANCE_OFFSET;
    let previous_balance = read_u32(host, balance_address, "save_berry_balance")? as u64;
    let balance = adjust_balance(previous_balance, delta);
    write_u32(host, balance_address, balance as u32, "save_berry_balance")?;
    Ok(SaveBerryBalancePatch {
        save,
        previous_balance,
        balance,
    })
}

fn read_save_pointer(host: &OwnedHostApi) -> Result<usize, String> {
    let module_base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if module_base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, module_base + GLOBAL_ROOT_RVA, "global_root")?;
    if root == 0 {
        return Err("global root is null".to_string());
    }
    let owner = read_usize(host, root + GLOBAL_OWNER_OFFSET, "global_owner")?;
    if owner == 0 {
        return Err("global owner is null".to_string());
    }
    let save = read_usize(host, owner + SAVE_PTR_OFFSET, "save_state")?;
    if save == 0 {
        return Err("save state is null".to_string());
    }
    Ok(save)
}

fn read_u32(host: &OwnedHostApi, address: usize, label: &str) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_usize(host: &OwnedHostApi, address: usize, label: &str) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn write_u32(host: &OwnedHostApi, address: usize, value: u32, label: &str) -> Result<(), String> {
    host.memory()
        .write(address, &value.to_le_bytes())
        .map_err(|error| format!("{label} write failed address=0x{address:x}: {error}"))
}

fn adjust_balance(balance: u64, delta: i128) -> u64 {
    let adjusted = i128::from(balance) + delta;
    adjusted.clamp(0, i128::from(BERRY_BALANCE_CAP)) as u64
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
        std::slice::from_raw_parts(reward_out.cast_const(), REWARD_SLOT_COUNT)
            .try_into()
            .unwrap_or([0; REWARD_SLOT_COUNT])
    };
    Some(reward_commit_event_from_slots(rank_or_mode, &slots))
}

fn reward_commit_event_from_slots(
    rank_or_mode: i32,
    slots: &[u64; REWARD_SLOT_COUNT],
) -> RewardCommitEvent {
    let rank = RankValue::from_slot(rank_or_mode.clamp(0, u8::MAX as i32) as u8);
    RewardCommitEvent::new(rank, RewardState::new().with_berry(slots[BERRY_TOTAL_SLOT]))
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use crate::runtime::core::rewards::RewardState;

    use super::*;

    fn pending_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
    }

    #[test]
    fn reward_commit_event_uses_rank_and_berry_slot() {
        let slots = [0, 0, 0, 0, 0, 0, 321, 0];
        let event = reward_commit_event_from_slots(5, &slots);

        assert_eq!(event.rank.debug_alias(), "S+");
        assert_eq!(event.rewards, RewardState::new().with_berry(321));
    }

    #[test]
    fn pending_berry_total_is_taken_once() {
        let _lock = pending_test_lock();
        clear_pending_berry_total();
        request_berry_total(642);

        assert_eq!(take_pending_berry_total(), Some(642));
        assert_eq!(take_pending_berry_total(), None);
    }

    #[test]
    fn pending_berry_total_updates_reward_slot() {
        let _lock = pending_test_lock();
        let mut slots = [0_u64; REWARD_SLOT_COUNT];
        slots[BERRY_TOTAL_SLOT] = 321;
        slots[7] = 10_000;
        clear_pending_berry_total();
        request_berry_total(642);

        apply_pending_berry_total(slots.as_mut_ptr());

        assert_eq!(slots[BERRY_TOTAL_SLOT], 642);
        assert_eq!(slots[7], 10_000);
        assert_eq!(take_pending_berry_total(), None);
    }

    #[test]
    fn reward_buffer_berry_total_updates_total_and_reports_delta() {
        let mut slots = [0_u64; REWARD_SLOT_COUNT];
        slots[BERRY_TOTAL_SLOT] = 1_109_250;
        slots[7] = 23_345_600;

        let adjustment = unsafe { apply_reward_buffer_berry_total(slots.as_mut_ptr(), 2_218_500) }
            .expect("adjustment");

        assert_eq!(
            adjustment,
            BerryBalanceAdjustment {
                previous_total: 1_109_250,
                total: 2_218_500,
                delta: 1_109_250,
            }
        );
        assert_eq!(slots[BERRY_TOTAL_SLOT], 2_218_500);
        assert_eq!(slots[7], 23_345_600);
    }

    #[test]
    fn adjust_balance_clamps_like_game_balance() {
        assert_eq!(adjust_balance(100, -150), 0);
        assert_eq!(
            adjust_balance(BERRY_BALANCE_CAP - 10, 100),
            BERRY_BALANCE_CAP
        );
    }
}
