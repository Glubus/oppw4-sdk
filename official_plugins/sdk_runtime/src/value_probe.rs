use std::{collections::HashSet, thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::config::ValueProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

const GLOBAL_ROOT_RVA: usize = 0x1eba750;

const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValueSnapshot {
    global: usize,
    mission_id: u16,
    mode_type: u8,
    difficulty: u8,
    hits: Vec<ValueHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValueHit {
    value: u32,
    width: ValueWidth,
    offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueWidth {
    U16,
    U32,
}

pub(crate) fn start(host: OwnedHostApi, config: ValueProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "value_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_value_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: ValueProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "value_probe started interval_ms={} scan_bytes=0x{:x} max_hits={} values={:?}",
            interval.as_millis(),
            config.scan_bytes,
            config.max_hits,
            config.values,
        ),
    );

    let mut last_snapshot = None;
    let mut last_error = String::new();
    loop {
        thread::sleep(interval);
        match read_snapshot(&host, &config) {
            Ok(snapshot) => {
                if last_snapshot.as_ref() != Some(&snapshot) {
                    log_snapshot(&host, &snapshot);
                    last_snapshot = Some(snapshot);
                }
                last_error.clear();
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("value_probe pending: {error}"));
                    last_error = error;
                }
            }
        }
    }
}

fn read_snapshot(host: &OwnedHostApi, config: &ValueProbeConfig) -> Result<ValueSnapshot, String> {
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

    let mut bytes = vec![0u8; config.scan_bytes.clamp(4096, 0x100000)];
    read_exact(host, global, &mut bytes, "global_value_scan")?;

    Ok(ValueSnapshot {
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        hits: scan_values(&bytes, &config.values, config.max_hits),
    })
}

fn log_snapshot(host: &OwnedHostApi, snapshot: &ValueSnapshot) {
    let hits = if snapshot.hits.is_empty() {
        "none".to_string()
    } else {
        snapshot
            .hits
            .iter()
            .map(|hit| format_hit(hit))
            .collect::<Vec<_>>()
            .join(",")
    };
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "value_probe mission_id={} difficulty={} mode_type={} global=0x{:x} hits={hits}",
            snapshot.mission_id, snapshot.difficulty, snapshot.mode_type, snapshot.global,
        ),
    );
}

fn scan_values(bytes: &[u8], values: &[u32], max_hits: usize) -> Vec<ValueHit> {
    let targets = values.iter().copied().collect::<HashSet<_>>();
    let mut hits = Vec::new();
    for offset in (0..bytes.len().saturating_sub(1)).step_by(2) {
        let value = u32::from(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
        if targets.contains(&value) {
            hits.push(ValueHit {
                value,
                width: ValueWidth::U16,
                offset,
            });
        }
        if hits.len() >= max_hits {
            return hits;
        }
    }
    for offset in (0..bytes.len().saturating_sub(3)).step_by(4) {
        let value = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        if targets.contains(&value) {
            hits.push(ValueHit {
                value,
                width: ValueWidth::U32,
                offset,
            });
        }
        if hits.len() >= max_hits {
            break;
        }
    }
    hits
}

fn format_hit(hit: &ValueHit) -> String {
    let width = match hit.width {
        ValueWidth::U16 => "u16",
        ValueWidth::U32 => "u32",
    };
    format!("{}:{}@+0x{:x}", hit.value, width, hit.offset)
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
    fn scans_u16_and_u32_targets() {
        let mut bytes = vec![0u8; 16];
        bytes[2..4].copy_from_slice(&170u16.to_le_bytes());
        bytes[8..12].copy_from_slice(&487_000u32.to_le_bytes());

        let hits = scan_values(&bytes, &[170, 487_000], 8);

        assert_eq!(
            hits,
            vec![
                ValueHit {
                    value: 170,
                    width: ValueWidth::U16,
                    offset: 2,
                },
                ValueHit {
                    value: 487_000,
                    width: ValueWidth::U32,
                    offset: 8,
                },
            ]
        );
    }

    #[test]
    fn formats_hits_compactly() {
        let hit = ValueHit {
            value: 992_250,
            width: ValueWidth::U32,
            offset: 0x1234,
        };

        assert_eq!(format_hit(&hit), "992250:u32@+0x1234");
    }
}
