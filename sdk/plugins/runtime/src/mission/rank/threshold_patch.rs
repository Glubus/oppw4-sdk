use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::RankRuntimeConfig,
    runtime::{probe::PLUGIN_ID, reader::read_usize},
};

const FIXED_ROOT_RVA: usize = 0x1eba738;
const FIXED_TABLE_OWNER_OFFSET: usize = 0x18;
const FIXED_HELPER_TABLE_OFFSET: usize = 0x28;
const NORMAL_HELPER_ROW_BASE: usize = 0x4c;
const NORMAL_HELPER_ROW_STRIDE: usize = 0xdc;
const SLOT_SELECTOR_OFFSET: usize = 0x64;
const COUNT_SELECTOR: u16 = 1;
const RANK_THRESHOLD_COUNT: usize = 5;
const THRESHOLD_ROWS: [usize; RANK_THRESHOLD_COUNT] = [0x00, 0x0c, 0x18, 0x24, 0x30];
const MAX_PATCH_ATTEMPTS: usize = 60;
const PATCH_RETRY_MS: u64 = 500;

pub(crate) fn install(host: OwnedHostApi, config: RankRuntimeConfig) {
    if !config.shift_count_thresholds || config.shift_count_rank_row_ids.is_empty() {
        return;
    }

    let _ = thread::Builder::new()
        .name("oppw4_rank_threshold_patch".to_string())
        .spawn(move || run(host, config));
}

fn run(host: OwnedHostApi, config: RankRuntimeConfig) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_runtime fixed count threshold patch started row_ids={:?} source_prefix={:?} inserted_first={} inserted_second={:?}",
            config.shift_count_rank_row_ids,
            config.shift_count_source_prefix,
            config.shift_count_inserted_first,
            config.shift_count_inserted_second
        ),
    );

    let started_at = Instant::now();
    let mut last_error = String::new();
    for attempt in 1..=MAX_PATCH_ATTEMPTS {
        match patch_rows(&host, &config) {
            Ok(summary) => {
                let _ = host.log().write(
                    PLUGIN_ID,
                    format!(
                        "rank_runtime fixed count threshold patch done attempts={attempt} patched={} skipped={} elapsed_ms={}",
                        summary.patched,
                        summary.skipped,
                        started_at.elapsed().as_millis()
                    ),
                );
                return;
            }
            Err(error) => {
                if error != last_error {
                    let _ = host.log().write(
                        PLUGIN_ID,
                        format!("rank_runtime fixed count threshold patch pending: {error}"),
                    );
                    last_error = error;
                }
                thread::sleep(Duration::from_millis(PATCH_RETRY_MS));
            }
        }
    }

    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_runtime fixed count threshold patch failed after {MAX_PATCH_ATTEMPTS} attempts: {last_error}"
        ),
    );
}

fn patch_rows(host: &OwnedHostApi, config: &RankRuntimeConfig) -> Result<PatchSummary, String> {
    let table = read_fixed_helper_table(host)?;
    let mut summary = PatchSummary::default();
    for row_id in &config.shift_count_rank_row_ids {
        let row = helper_row_address(table, *row_id);
        let snapshot = read_count_thresholds(host, row)?;
        if !snapshot.matches_prefix(config.shift_count_source_prefix) {
            summary.skipped += 1;
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_runtime fixed count threshold skip row_id={} row=0x{row:x} thresholds=[{}] expected_prefix=[{}]",
                    row_id,
                    csv(snapshot.thresholds),
                    csv(config.shift_count_source_prefix)
                ),
            );
            continue;
        }

        let patched = config.count_threshold_override.unwrap_or_else(|| {
            snapshot.shifted(
                config.shift_count_inserted_first,
                config.shift_count_inserted_second,
            )
        });
        write_thresholds(host, row, snapshot.slot, patched)?;
        summary.patched += 1;
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_runtime fixed count thresholds patched row_id={} row=0x{row:x} slot={} old=[{}] new=[{}] mode={}",
                row_id,
                snapshot.slot,
                csv(snapshot.thresholds),
                csv(patched),
                if config.count_threshold_override.is_some() {
                    "override"
                } else {
                    "shift"
                }
            ),
        );
    }
    Ok(summary)
}

fn read_fixed_helper_table(host: &OwnedHostApi) -> Result<usize, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, base + FIXED_ROOT_RVA, "fixed_root")?;
    let owner = read_usize(host, root + FIXED_TABLE_OWNER_OFFSET, "fixed_root+0x18")?;
    let table = read_usize(
        host,
        owner + FIXED_HELPER_TABLE_OFFSET,
        "fixed_helper_table_owner+0x28",
    )?;
    if table == 0 {
        return Err("fixed helper table is null".to_string());
    }
    Ok(table)
}

fn helper_row_address(table: usize, row_id: u16) -> usize {
    table + NORMAL_HELPER_ROW_BASE + usize::from(row_id) * NORMAL_HELPER_ROW_STRIDE
}

fn read_count_thresholds(host: &OwnedHostApi, row: usize) -> Result<ThresholdSnapshot, String> {
    let selectors = [
        read_u16(host, row + SLOT_SELECTOR_OFFSET, "rank_helper_selectors")?,
        read_u16(
            host,
            row + SLOT_SELECTOR_OFFSET + 2,
            "rank_helper_selectors",
        )?,
        read_u16(
            host,
            row + SLOT_SELECTOR_OFFSET + 4,
            "rank_helper_selectors",
        )?,
    ];
    let slot = selectors
        .iter()
        .position(|selector| *selector == COUNT_SELECTOR)
        .ok_or_else(|| {
            format!(
                "rank helper row=0x{row:x} has no count selector selectors=[{}]",
                csv(selectors)
            )
        })?;

    let mut thresholds = [0; RANK_THRESHOLD_COUNT];
    for (output, offset) in thresholds.iter_mut().zip(THRESHOLD_ROWS) {
        *output = read_u32(
            host,
            row + offset + slot * size_of::<u32>(),
            "rank_helper_count_threshold",
        )?;
    }

    Ok(ThresholdSnapshot { slot, thresholds })
}

fn write_thresholds(
    host: &OwnedHostApi,
    row: usize,
    slot: usize,
    thresholds: [u32; RANK_THRESHOLD_COUNT],
) -> Result<(), String> {
    for (offset, value) in THRESHOLD_ROWS.into_iter().zip(thresholds) {
        let address = row + offset + slot * size_of::<u32>();
        host.memory()
            .write(address, &value.to_le_bytes())
            .map_err(|error| {
                format!("rank helper threshold write failed address=0x{address:x}: {error}")
            })?;
    }
    Ok(())
}

fn read_u16(host: &OwnedHostApi, address: usize, label: &str) -> Result<u16, String> {
    let mut bytes = [0; 2];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(host: &OwnedHostApi, address: usize, label: &str) -> Result<u32, String> {
    let mut bytes = [0; 4];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))?;
    Ok(u32::from_le_bytes(bytes))
}

fn csv<const N: usize, T: ToString>(values: [T; N]) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PatchSummary {
    patched: usize,
    skipped: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThresholdSnapshot {
    slot: usize,
    thresholds: [u32; RANK_THRESHOLD_COUNT],
}

impl ThresholdSnapshot {
    fn matches_prefix(self, prefix: [u32; 3]) -> bool {
        self.thresholds[..prefix.len()] == prefix
    }

    fn shifted(
        self,
        inserted_first: u32,
        inserted_second: Option<u32>,
    ) -> [u32; RANK_THRESHOLD_COUNT] {
        [
            inserted_first,
            inserted_second.unwrap_or(self.thresholds[0]),
            self.thresholds[1],
            self.thresholds[2],
            self.thresholds[3],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_normal_helper_row_address() {
        assert_eq!(helper_row_address(0x1000, 0), 0x104c);
        assert_eq!(helper_row_address(0x1000, 35), 0x1000 + 0x4c + 35 * 0xdc);
    }

    #[test]
    fn shifts_thresholds_right() {
        let snapshot = ThresholdSnapshot {
            slot: 1,
            thresholds: [60_000, 60_000, 48_000, 42_000, 30_000],
        };

        assert!(snapshot.matches_prefix([60_000, 60_000, 48_000]));
        assert_eq!(
            snapshot.shifted(72_000, None),
            [72_000, 60_000, 60_000, 48_000, 42_000]
        );
        assert_eq!(
            snapshot.shifted(72_000, Some(72_000)),
            [72_000, 72_000, 60_000, 48_000, 42_000]
        );
    }
}
