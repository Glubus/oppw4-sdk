use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::config::PlayerResultProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const SAVE_PTR_OFFSET: usize = 0x10;
const GLOBAL_PTR_OFFSET: usize = 0x28;

const ACTIVE_PLAYER_OFFSET: usize = 0x31;
const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;

const PLAYER_STRIDE: usize = 0xb90;
const PLAYER_RESULT_STAT_OFFSETS: [usize; 12] = [
    0x8fc, 0x900, 0x904, 0x908, 0x90c, 0x910, 0x914, 0x918, 0x91c, 0x920, 0x924, 0x928,
];
const SAVE_RESULT_TOTAL_OFFSETS: [usize; 12] = [
    0x764, 0x768, 0x76c, 0x770, 0x774, 0x778, 0x77c, 0x780, 0x784, 0x788, 0x78c, 0x790,
];
const SAVE_MODE_TOTAL_OFFSETS: [usize; 3] = [0x794, 0x798, 0x79c];
const SOUL_STATE_OFFSET: usize = 0xfe6c;
const SOUL_STATE_WORDS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlayerResultSnapshot {
    global: usize,
    save: usize,
    active_player: u8,
    mission_id: u16,
    mode_type: u8,
    difficulty: u8,
    player_stats: [[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4],
    save_totals: [u32; SAVE_RESULT_TOTAL_OFFSETS.len()],
    save_mode_totals: [u32; SAVE_MODE_TOTAL_OFFSETS.len()],
    soul_state: [u32; SOUL_STATE_WORDS],
}

pub(crate) fn start(host: OwnedHostApi, config: PlayerResultProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "player_result_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_player_result_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: PlayerResultProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "player_result_probe started interval_ms={} snapshot_interval_ms={}",
            interval.as_millis(),
            config.snapshot_interval_ms,
        ),
    );

    let snapshot_interval = snapshot_interval(config.snapshot_interval_ms);
    let mut last_snapshot_at = Instant::now()
        .checked_sub(snapshot_interval.unwrap_or(Duration::ZERO))
        .unwrap_or_else(Instant::now);
    let mut last_snapshot = None;
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
            }
            Err(error) => {
                if error != last_error || last_error_at.elapsed() >= Duration::from_secs(10) {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("player_result_probe pending: {error}"));
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

fn read_snapshot(host: &OwnedHostApi) -> Result<PlayerResultSnapshot, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, base + GLOBAL_ROOT_RVA, "global_root")?;
    let first = read_usize(host, root + GLOBAL_PTR_FIRST_OFFSET, "global_root+0x18")?;
    let save = read_usize(host, first + SAVE_PTR_OFFSET, "save_state")?;
    let global = read_usize(host, first + GLOBAL_PTR_OFFSET, "global_state")?;
    if save == 0 {
        return Err("save_state is null".to_string());
    }
    if global == 0 {
        return Err("global_state is null".to_string());
    }

    Ok(PlayerResultSnapshot {
        global,
        save,
        active_player: read_u8(host, global + ACTIVE_PLAYER_OFFSET, "active_player")?,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        player_stats: read_player_stats(host, global)?,
        save_totals: read_u32_offsets(host, save, &SAVE_RESULT_TOTAL_OFFSETS)?,
        save_mode_totals: read_u32_offsets(host, save, &SAVE_MODE_TOTAL_OFFSETS)?,
        soul_state: read_soul_state(host, save)?,
    })
}

fn read_player_stats(
    host: &OwnedHostApi,
    global: usize,
) -> Result<[[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4], String> {
    let mut players = [[0u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4];
    for (player_index, stats) in players.iter_mut().enumerate() {
        let player_base = global + player_index * PLAYER_STRIDE;
        *stats = read_u32_offsets(host, player_base, &PLAYER_RESULT_STAT_OFFSETS)?;
    }
    Ok(players)
}

fn read_soul_state(host: &OwnedHostApi, save: usize) -> Result<[u32; SOUL_STATE_WORDS], String> {
    let mut values = [0u32; SOUL_STATE_WORDS];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(
            host,
            save + SOUL_STATE_OFFSET + index * size_of::<u32>(),
            "soul_state",
        )?;
    }
    Ok(values)
}

fn read_u32_offsets<const N: usize>(
    host: &OwnedHostApi,
    base: usize,
    offsets: &[usize; N],
) -> Result<[u32; N], String> {
    let mut values = [0u32; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(host, base + offsets[index], "u32_offset")?;
    }
    Ok(values)
}

fn log_snapshot(host: &OwnedHostApi, snapshot: PlayerResultSnapshot) {
    let active_index = usize::from(snapshot.active_player).min(3);
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "player_result_probe mission_id={} difficulty={} mode_type={} active_player={} global=0x{:x} save=0x{:x} active_stats={} all_players={} save_totals={} save_mode_totals={} soul_state={}",
            snapshot.mission_id,
            snapshot.difficulty,
            snapshot.mode_type,
            snapshot.active_player,
            snapshot.global,
            snapshot.save,
            format_named_values(
                &PLAYER_RESULT_STAT_OFFSETS,
                &snapshot.player_stats[active_index],
            ),
            format_players(&snapshot.player_stats),
            format_named_values(&SAVE_RESULT_TOTAL_OFFSETS, &snapshot.save_totals),
            format_named_values(&SAVE_MODE_TOTAL_OFFSETS, &snapshot.save_mode_totals),
            format_soul_state(&snapshot.soul_state),
        ),
    );
}

fn format_players(players: &[[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4]) -> String {
    players
        .iter()
        .enumerate()
        .map(|(index, stats)| {
            format!(
                "p{}:[{}]",
                index,
                stats
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn format_named_values<const N: usize>(offsets: &[usize; N], values: &[u32; N]) -> String {
    offsets
        .iter()
        .zip(values)
        .map(|(offset, value)| format!("+0x{offset:x}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_soul_state(values: &[u32; SOUL_STATE_WORDS]) -> String {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value != 0)
        .map(|(index, value)| format!("+0x{:x}:{value}", SOUL_STATE_OFFSET + index * 4))
        .collect::<Vec<_>>()
        .join(",")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_interval_zero_disables_periodic_logging() {
        assert_eq!(snapshot_interval(0), None);
    }

    #[test]
    fn snapshot_interval_has_minimum_for_log_safety() {
        assert_eq!(snapshot_interval(1), Some(Duration::from_millis(250)));
        assert_eq!(snapshot_interval(1000), Some(Duration::from_millis(1000)));
    }

    #[test]
    fn formats_empty_soul_state_as_empty_string() {
        assert_eq!(format_soul_state(&[0; SOUL_STATE_WORDS]), "");
    }

    #[test]
    fn formats_named_offsets() {
        assert_eq!(
            format_named_values(&[0x8fc, 0x900], &[30, 200]),
            "+0x8fc:30,+0x900:200"
        );
    }
}
