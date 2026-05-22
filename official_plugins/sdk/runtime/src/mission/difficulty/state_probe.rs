use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::DifficultyProbeConfig,
    mission::difficulty::reward_row::{read_reward_row_dump, RewardRowDump},
};

const PLUGIN_ID: &str = "sdk_runtime";

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const CACHED_MISSION_RVA: usize = 0x1e5ec00;
const CACHED_DIFFICULTY_RVA: usize = 0x1e5ec04;

const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const RESULT_MODE_OFFSET: usize = 0x1d753;
const REWARD_MODE_OFFSET: usize = 0x1d754;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const SPECIAL_FLAG_OFFSET: usize = 0x1d762;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DifficultySnapshot {
    module_base: usize,
    global: usize,
    mission_id: u16,
    mode_type: u8,
    reward_mode: u8,
    difficulty: u8,
    special_flag: u8,
    cached_mission: u32,
    cached_difficulty: u32,
}

pub(crate) fn start(host: OwnedHostApi, config: DifficultyProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "difficulty_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(50));
    let _ = thread::Builder::new()
        .name("oppw4_difficulty_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: DifficultyProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "difficulty_probe started interval_ms={} dump_reward_row={} snapshot_interval_ms={}",
            interval.as_millis(),
            config.dump_reward_row,
            config.snapshot_interval_ms,
        ),
    );

    let snapshot_interval = snapshot_interval(config.snapshot_interval_ms);
    let mut last_snapshot_at = Instant::now()
        .checked_sub(snapshot_interval.unwrap_or(Duration::ZERO))
        .unwrap_or_else(Instant::now);
    let mut last_snapshot = None;
    let mut last_reward_row = None;
    let mut last_error = String::new();
    let mut last_error_at = Instant::now() - Duration::from_secs(60);
    loop {
        thread::sleep(interval);
        match read_snapshot(&host) {
            Ok(snapshot) => {
                last_error.clear();
                let changed = last_snapshot != Some(snapshot);
                let periodic = snapshot_interval
                    .is_some_and(|interval| last_snapshot_at.elapsed() >= interval);
                if !changed && !periodic {
                    continue;
                }
                last_snapshot = Some(snapshot);
                last_snapshot_at = Instant::now();
                log_snapshot(&host, snapshot);
                if config.dump_reward_row {
                    log_reward_row(&host, snapshot, &mut last_reward_row, periodic);
                }
            }
            Err(error) => {
                if error != last_error || last_error_at.elapsed() >= Duration::from_secs(10) {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("difficulty_probe pending: {error}"));
                    last_error = error;
                    last_error_at = Instant::now();
                }
            }
        }
    }
}

fn snapshot_interval(interval_ms: u64) -> Option<Duration> {
    if interval_ms == 0 {
        None
    } else {
        Some(Duration::from_millis(interval_ms.max(250)))
    }
}

fn read_snapshot(host: &OwnedHostApi) -> Result<DifficultySnapshot, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, base + GLOBAL_ROOT_RVA, "global_root")?;
    let first = read_usize(host, root + GLOBAL_PTR_FIRST_OFFSET, "global_root+0x18")?;
    let global = read_usize(host, first + GLOBAL_PTR_SECOND_OFFSET, "global_state")?;
    if global == 0 {
        return Err("global_state is null".to_string());
    }

    Ok(DifficultySnapshot {
        module_base: base,
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + RESULT_MODE_OFFSET, "mode_type")?,
        reward_mode: read_u8(host, global + REWARD_MODE_OFFSET, "reward_mode")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        special_flag: read_u8(host, global + SPECIAL_FLAG_OFFSET, "special_flag")?,
        cached_mission: read_u32(host, base + CACHED_MISSION_RVA, "cached_mission")?,
        cached_difficulty: read_u32(host, base + CACHED_DIFFICULTY_RVA, "cached_difficulty")?,
    })
}

fn log_snapshot(host: &OwnedHostApi, snapshot: DifficultySnapshot) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "difficulty_probe mission_id={} difficulty={}({}) mode_type={}({}) reward_mode={} special_flag={} cached_mission={} cached_difficulty={} global=0x{:x}",
            snapshot.mission_id,
            snapshot.difficulty,
            difficulty_label(snapshot.difficulty),
            snapshot.mode_type,
            mode_type_label(snapshot.mode_type),
            snapshot.reward_mode,
            snapshot.special_flag,
            snapshot.cached_mission,
            snapshot.cached_difficulty,
            snapshot.global,
        ),
    );
}

fn log_reward_row(
    host: &OwnedHostApi,
    snapshot: DifficultySnapshot,
    last_reward_row: &mut Option<RewardRowDump>,
    force: bool,
) {
    match read_reward_row_dump(
        host,
        snapshot.module_base,
        snapshot.mission_id,
        snapshot.difficulty,
    ) {
        Ok(Some(row)) => {
            if !force && last_reward_row.as_ref() == Some(&row) {
                return;
            }
            let log = row.format_log();
            *last_reward_row = Some(row);
            let _ = host.log().write(PLUGIN_ID, log);
        }
        Ok(None) => {
            *last_reward_row = None;
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "reward_row unavailable mission_id={} difficulty={} reason=outside_base_reward_index",
                    snapshot.mission_id, snapshot.difficulty
                ),
            );
        }
        Err(error) => {
            *last_reward_row = None;
            let _ = host
                .log()
                .write(PLUGIN_ID, format!("reward_row pending: {error}"));
        }
    }
}

fn read_u8(host: &OwnedHostApi, address: usize, label: &str) -> Result<u8, String> {
    let mut bytes = [0u8; 1];
    read_exact(host, address, &mut bytes, label)?;
    Ok(bytes[0])
}

fn read_u16(host: &OwnedHostApi, address: usize, label: &str) -> Result<u16, String> {
    let mut bytes = [0u8; 2];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(host: &OwnedHostApi, address: usize, label: &str) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_usize(host: &OwnedHostApi, address: usize, label: &str) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn read_exact(
    host: &OwnedHostApi,
    address: usize,
    out: &mut [u8],
    label: &str,
) -> Result<(), String> {
    host.memory()
        .read(address, out)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))
}

fn difficulty_label(value: u8) -> &'static str {
    match value {
        0 => "easy",
        1 => "normal",
        2 => "hard",
        3 => "super_hard",
        _ => "unknown",
    }
}

fn mode_type_label(value: u8) -> &'static str {
    match value {
        0 => "story",
        1 => "free_log",
        2 => "treasure_log",
        3 => "unknown_dlc_or_special_3",
        4 => "unknown_dlc_or_special_4",
        5 => "unknown_dlc_or_special_5",
        6 => "inactive_or_transition",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_known_difficulty_values() {
        assert_eq!(difficulty_label(0), "easy");
        assert_eq!(difficulty_label(1), "normal");
        assert_eq!(difficulty_label(2), "hard");
        assert_eq!(difficulty_label(3), "super_hard");
        assert_eq!(difficulty_label(4), "unknown");
    }

    #[test]
    fn labels_observed_mode_types() {
        assert_eq!(mode_type_label(0), "story");
        assert_eq!(mode_type_label(1), "free_log");
        assert_eq!(mode_type_label(2), "treasure_log");
        assert_eq!(mode_type_label(6), "inactive_or_transition");
        assert_eq!(mode_type_label(9), "unknown");
    }

    #[test]
    fn snapshot_interval_zero_disables_periodic_logging() {
        assert_eq!(snapshot_interval(0), None);
    }

    #[test]
    fn snapshot_interval_has_minimum_for_log_safety() {
        assert_eq!(snapshot_interval(1), Some(Duration::from_millis(250)));
        assert_eq!(snapshot_interval(1000), Some(Duration::from_millis(1000)));
    }
}
