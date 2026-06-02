use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    OnceLock,
};

use plugin_sdk::OwnedHostApi;

const PLUGIN_ID: &str = "sdk_runtime";
pub(super) const REWARD_SLOT_COUNT: usize = 8;
pub(super) const BERRY_TOTAL_SLOT: usize = 6;
const BERRY_BALANCE_CAP: u64 = 999_999_999;
const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_OWNER_OFFSET: usize = 0x18;
const SAVE_PTR_OFFSET: usize = 0x10;
const SAVE_BERRY_BALANCE_OFFSET: usize = 0x14;

static PENDING_BERRY_TOTAL_SET: AtomicBool = AtomicBool::new(false);
static PENDING_BERRY_TOTAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn request_berry_total(total: u64) {
    PENDING_BERRY_TOTAL.store(total, Ordering::Relaxed);
    PENDING_BERRY_TOTAL_SET.store(true, Ordering::Release);
}

pub(super) fn clear_pending_berry_total() {
    PENDING_BERRY_TOTAL_SET.store(false, Ordering::Release);
}

pub(super) fn apply_pending_berry_total(reward_out: *mut u64, host: Option<&OwnedHostApi>) {
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
    apply_save_berry_balance_delta(host, adjustment.delta);
}

fn take_pending_berry_total() -> Option<u64> {
    PENDING_BERRY_TOTAL_SET
        .swap(false, Ordering::AcqRel)
        .then(|| PENDING_BERRY_TOTAL.load(Ordering::Relaxed))
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

fn apply_save_berry_balance_delta(host: Option<&OwnedHostApi>, delta: i128) {
    if delta == 0 {
        return;
    }
    let Some(host) = host else {
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

#[cfg(test)]
pub(super) fn pending_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("lock")
}

#[cfg(test)]
mod tests {
    use super::*;

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

        apply_pending_berry_total(slots.as_mut_ptr(), None);

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
