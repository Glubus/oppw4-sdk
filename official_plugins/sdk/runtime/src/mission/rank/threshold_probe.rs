use std::{
    fmt::Write,
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::config::RankThresholdProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const FIXED_ROOT_RVA: usize = 0x1eba738;

const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;
const FIXED_TABLE_OWNER_OFFSET: usize = 0x18;
const FIXED_RANK_TABLE_OFFSET: usize = 0x8;

const ACTIVE_PLAYER_OFFSET: usize = 0x31;
const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const RESULT_RANK_AREA_OFFSET: usize = 0x1d9b0;
const RESULT_RANK_SLOT_STRIDE: usize = 0x50;
const RESULT_RANK_SLOT_COUNT: usize = 4;
const RESULT_RANK_SLOT_WORDS: usize = RESULT_RANK_SLOT_STRIDE / 2;

const FIXED_RANK_ROW_STRIDE: usize = 0x44;
const FIXED_RANK_ROW_WORDS: usize = FIXED_RANK_ROW_STRIDE / 2;
const FIXED_CONDITION_TABLE_OFFSET: usize = 0xc43c;
const FIXED_CONDITION_ROW_STRIDE: usize = 0x34;
const FIXED_CONDITION_ROW_WORDS: usize = FIXED_CONDITION_ROW_STRIDE / 2;
const KNOWN_CONDITION_ROW_LIMIT: u16 = 0x68;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RankThresholdSnapshot {
    global: usize,
    fixed_rank_table: usize,
    active_player: u8,
    mission_id: u16,
    mode_type: u8,
    difficulty: u8,
    slots: [RankSlot; RESULT_RANK_SLOT_COUNT],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RankSlot {
    slot_index: usize,
    rank_row_id: u16,
    raw_words: [u16; RESULT_RANK_SLOT_WORDS],
    fixed_row_words: Option<[u16; FIXED_RANK_ROW_WORDS]>,
    condition_row_id: Option<u16>,
    condition_row_words: Option<[u16; FIXED_CONDITION_ROW_WORDS]>,
}

pub(crate) fn start(host: OwnedHostApi, config: RankThresholdProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_threshold_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_rank_threshold_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: RankThresholdProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_threshold_probe started interval_ms={} snapshot_interval_ms={}",
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
                let changed = last_snapshot.as_ref() != Some(&snapshot);
                let periodic = snapshot_interval
                    .is_some_and(|interval| last_snapshot_at.elapsed() >= interval);
                if !changed && !periodic {
                    continue;
                }

                last_snapshot = Some(snapshot.clone());
                last_snapshot_at = Instant::now();
                let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
            }
            Err(error) => {
                if error != last_error || last_error_at.elapsed() >= Duration::from_secs(10) {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("rank_threshold_probe pending: {error}"));
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

fn read_snapshot(host: &OwnedHostApi) -> Result<RankThresholdSnapshot, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let global = read_global_state(host, base)?;
    let fixed_rank_table = read_fixed_rank_table(host, base)?;
    if fixed_rank_table == 0 {
        return Err("fixed rank table is null".to_string());
    }

    Ok(RankThresholdSnapshot {
        global,
        fixed_rank_table,
        active_player: read_u8(host, global + ACTIVE_PLAYER_OFFSET, "active_player")?,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        slots: read_slots(host, global, fixed_rank_table)?,
    })
}

fn read_global_state(host: &OwnedHostApi, base: usize) -> Result<usize, String> {
    let root = read_usize(host, base + GLOBAL_ROOT_RVA, "global_root")?;
    let first = read_usize(host, root + GLOBAL_PTR_FIRST_OFFSET, "global_root+0x18")?;
    let global = read_usize(host, first + GLOBAL_PTR_SECOND_OFFSET, "global_state")?;
    if global == 0 {
        return Err("global_state is null".to_string());
    }
    Ok(global)
}

fn read_fixed_rank_table(host: &OwnedHostApi, base: usize) -> Result<usize, String> {
    let root = read_usize(host, base + FIXED_ROOT_RVA, "fixed_root")?;
    let owner = read_usize(host, root + FIXED_TABLE_OWNER_OFFSET, "fixed_root+0x18")?;
    read_usize(
        host,
        owner + FIXED_RANK_TABLE_OFFSET,
        "fixed_rank_table_owner+0x8",
    )
}

fn read_slots(
    host: &OwnedHostApi,
    global: usize,
    fixed_rank_table: usize,
) -> Result<[RankSlot; RESULT_RANK_SLOT_COUNT], String> {
    let mut slots = Vec::with_capacity(RESULT_RANK_SLOT_COUNT);
    for slot_index in 0..RESULT_RANK_SLOT_COUNT {
        let slot_address = global + RESULT_RANK_AREA_OFFSET + slot_index * RESULT_RANK_SLOT_STRIDE;
        let raw_words = read_u16_block::<RESULT_RANK_SLOT_WORDS>(host, slot_address, "rank_slot")?;
        let rank_row_id = raw_words[0];
        let fixed_row_words = read_fixed_rank_row(host, fixed_rank_table, rank_row_id).ok();
        let condition_row_id = fixed_row_words.and_then(condition_row_id);
        let condition_row_words = condition_row_id.and_then(|row_id| {
            read_condition_row(host, fixed_rank_table, row_id)
                .ok()
                .flatten()
        });

        slots.push(RankSlot {
            slot_index,
            rank_row_id,
            raw_words,
            fixed_row_words,
            condition_row_id,
            condition_row_words,
        });
    }

    slots
        .try_into()
        .map_err(|_| "rank slot count mismatch".to_string())
}

fn read_fixed_rank_row(
    host: &OwnedHostApi,
    fixed_rank_table: usize,
    rank_row_id: u16,
) -> Result<[u16; FIXED_RANK_ROW_WORDS], String> {
    let address = fixed_rank_table + usize::from(rank_row_id) * FIXED_RANK_ROW_STRIDE;
    read_u16_block::<FIXED_RANK_ROW_WORDS>(host, address, "fixed_rank_row")
}

fn condition_row_id(row: [u16; FIXED_RANK_ROW_WORDS]) -> Option<u16> {
    let value = row[0x16 / 2];
    if value == u16::MAX {
        None
    } else {
        Some(value)
    }
}

fn read_condition_row(
    host: &OwnedHostApi,
    fixed_rank_table: usize,
    row_id: u16,
) -> Result<Option<[u16; FIXED_CONDITION_ROW_WORDS]>, String> {
    if row_id >= KNOWN_CONDITION_ROW_LIMIT {
        return Ok(None);
    }
    let address = fixed_rank_table
        + FIXED_CONDITION_TABLE_OFFSET
        + usize::from(row_id) * FIXED_CONDITION_ROW_STRIDE;
    read_u16_block::<FIXED_CONDITION_ROW_WORDS>(host, address, "rank_condition_row").map(Some)
}

impl RankThresholdSnapshot {
    fn format_log(&self) -> String {
        format!(
            "rank_threshold_probe mission_id={} difficulty={} mode_type={} active_player={} global=0x{:x} fixed_rank_table=0x{:x} slots={}",
            self.mission_id,
            self.difficulty,
            self.mode_type,
            self.active_player,
            self.global,
            self.fixed_rank_table,
            self.slots
                .iter()
                .map(RankSlot::format_log)
                .collect::<Vec<_>>()
                .join(";"),
        )
    }
}

impl RankSlot {
    fn format_log(&self) -> String {
        format!(
            "p{}:rank_row={} raw=[{}] fixed=[{}] condition_row={} condition=[{}]",
            self.slot_index,
            self.rank_row_id,
            format_u16s(&self.raw_words),
            self.fixed_row_words
                .as_ref()
                .map_or_else(|| "unreadable".to_string(), |row| format_u16s(row)),
            self.condition_row_id
                .map_or_else(|| "none".to_string(), |value| value.to_string()),
            self.condition_row_words
                .as_ref()
                .map_or_else(|| "none".to_string(), |row| format_u16s(row)),
        )
    }
}

fn format_u16s(values: &[u16]) -> String {
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "+0x{:x}:{value}", index * 2);
    }
    text
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

fn read_usize(host: &OwnedHostApi, address: usize, label: &str) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn read_u16_block<const N: usize>(
    host: &OwnedHostApi,
    address: usize,
    label: &str,
) -> Result<[u16; N], String> {
    let mut bytes = vec![0u8; N * size_of::<u16>()];
    read_exact(host, address, &mut bytes, label)?;

    let mut values = [0u16; N];
    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
        values[index] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }
    Ok(values)
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
    fn formats_u16_offsets() {
        assert_eq!(format_u16s(&[7, 42, 65535]), "+0x0:7,+0x2:42,+0x4:65535");
    }

    #[test]
    fn extracts_condition_row_id_from_fixed_row() {
        let mut row = [0u16; FIXED_RANK_ROW_WORDS];
        row[0x16 / 2] = 12;
        assert_eq!(condition_row_id(row), Some(12));

        row[0x16 / 2] = u16::MAX;
        assert_eq!(condition_row_id(row), None);
    }
}
