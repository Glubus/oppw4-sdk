use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicU32, Ordering},
        OnceLock,
    },
    time::Instant,
};

use plugin_sdk::Oppw4GameStatus;

pub(crate) const GAME_PHASE_BOOTING: u32 = 1;
pub(crate) const GAME_PHASE_RDB_LOADING: u32 = 2;
pub(crate) const GAME_PHASE_RDB_BIN_LOADING: u32 = 3;
pub(crate) const GAME_PHASE_DLC_CHARACTER_LOADING: u32 = 4;
pub(crate) const GAME_PHASE_VIRTUAL_RESOURCE_LOADING: u32 = 5;

const GAME_FLAG_DLC_CHARACTER_SEEN: u32 = 1 << 0;
const GAME_FLAG_VIRTUAL_RESOURCE_SEEN: u32 = 1 << 1;

static START: OnceLock<Instant> = OnceLock::new();
static PHASE: AtomicU32 = AtomicU32::new(GAME_PHASE_BOOTING);
static FLAGS: AtomicU32 = AtomicU32::new(0);
static FILE_OPENS: AtomicU32 = AtomicU32::new(0);
static DLC_MARKERS: AtomicU32 = AtomicU32::new(0);
static VIRTUAL_MARKERS: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GameStatus {
    phase: u32,
    flags: u32,
    observed_file_opens: u32,
    seconds_since_host_start: u32,
}

pub(crate) fn initialize() {
    let _ = hooks::set_file_open_observer(observe_file_open);
}

pub(crate) unsafe extern "system" fn read_game_status(
    _provider_context: *mut c_void,
    out_status: *mut Oppw4GameStatus,
) -> i32 {
    let Some(out_status) = (unsafe { out_status.as_mut() }) else {
        return -1;
    };
    *out_status = game_status_to_abi(game_status());
    0
}

fn observe_file_open(path: &str) {
    START.get_or_init(Instant::now);
    FILE_OPENS.fetch_add(1, Ordering::Relaxed);

    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".rdb.bin") || lower.contains(".rdb.bin") {
        advance_phase(GAME_PHASE_RDB_BIN_LOADING, path);
    } else if lower.contains("file\\dlc\\dlc_character_") {
        mark_event(
            GAME_FLAG_DLC_CHARACTER_SEEN,
            &DLC_MARKERS,
            "dlc_character_seen",
            path,
        );
    } else if lower.contains("data\\0x") && lower.ends_with(".file") {
        mark_event(
            GAME_FLAG_VIRTUAL_RESOURCE_SEEN,
            &VIRTUAL_MARKERS,
            "virtual_resource_seen",
            path,
        );
    } else if lower.ends_with(".rdb") {
        advance_phase(GAME_PHASE_RDB_LOADING, path);
    }
}

fn game_status() -> GameStatus {
    let start = START.get_or_init(Instant::now);
    GameStatus {
        phase: PHASE.load(Ordering::Relaxed),
        flags: FLAGS.load(Ordering::Relaxed),
        observed_file_opens: FILE_OPENS.load(Ordering::Relaxed),
        seconds_since_host_start: start.elapsed().as_secs().min(u32::MAX as u64) as u32,
    }
}

fn game_status_to_abi(status: GameStatus) -> Oppw4GameStatus {
    Oppw4GameStatus {
        phase: status.phase,
        flags: status.flags,
        observed_file_opens: status.observed_file_opens,
        seconds_since_host_start: status.seconds_since_host_start,
    }
}

fn mark_event(flag: u32, counter: &AtomicU32, marker: &str, path: &str) {
    FLAGS.fetch_or(flag, Ordering::Relaxed);
    if !hooks_diagnostics_enabled() {
        return;
    }
    let index = counter.fetch_add(1, Ordering::Relaxed);
    if index < 12 {
        let _ = format!(
            "game_status marker {marker} count={} phase={} flags=0x{:x} after {} file opens path={}",
            index + 1,
            phase_name(PHASE.load(Ordering::Relaxed)),
            FLAGS.load(Ordering::Relaxed),
            FILE_OPENS.load(Ordering::Relaxed),
            path
        );
    } else if index == 12 {
        let _ = marker;
    }
}

fn advance_phase(next: u32, path: &str) {
    let mut current = PHASE.load(Ordering::Relaxed);
    while next > current {
        match PHASE.compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => {
                if hooks_diagnostics_enabled() {
                    let _ = format!(
                        "game_status phase {} -> {} after {} file opens path={}",
                        phase_name(current),
                        phase_name(next),
                        FILE_OPENS.load(Ordering::Relaxed),
                        path
                    );
                }
                return;
            }
            Err(actual) => current = actual,
        }
    }
}

fn hooks_diagnostics_enabled() -> bool {
    hooks::diagnostics_enabled()
}

fn phase_name(phase: u32) -> &'static str {
    match phase {
        GAME_PHASE_BOOTING => "booting",
        GAME_PHASE_RDB_LOADING => "rdb_loading",
        GAME_PHASE_RDB_BIN_LOADING => "rdb_bin_loading",
        GAME_PHASE_DLC_CHARACTER_LOADING => "dlc_character_loading",
        GAME_PHASE_VIRTUAL_RESOURCE_LOADING => "virtual_resource_loading",
        _ => "unknown",
    }
}
