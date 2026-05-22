use std::{thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::config::ResultProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

const GLOBAL_ROOT_RVA: usize = 0x1eba750;

const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;

const CONTEXT_OFFSET: usize = 0x244;
const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const REWARD_MODE_OFFSET: usize = 0x1d754;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const RESULT_AREA_OFFSET: usize = 0x1d9b0;
const RESULT_WORK_FLAG_OFFSET: usize = 0x1dafc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResultSnapshot {
    global: usize,
    mission_id: u16,
    mode_type: u8,
    reward_mode: u8,
    difficulty: u8,
    context: u32,
    work_flag: u32,
    area_hash: u64,
    area: Vec<u8>,
}

pub(crate) fn start(host: OwnedHostApi, config: ResultProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "result_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_result_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: ResultProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "result_probe started interval_ms={} result_area_bytes={} max_changed_words={}",
            interval.as_millis(),
            config.result_area_bytes,
            config.max_changed_words,
        ),
    );

    let mut last_snapshot = None;
    let mut last_error = String::new();
    loop {
        thread::sleep(interval);
        match read_snapshot(&host, config.result_area_bytes) {
            Ok(snapshot) => {
                if last_snapshot.as_ref() != Some(&snapshot) {
                    log_snapshot(
                        &host,
                        last_snapshot.as_ref(),
                        &snapshot,
                        config.max_changed_words,
                    );
                    last_snapshot = Some(snapshot);
                }
                last_error.clear();
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("result_probe pending: {error}"));
                    last_error = error;
                }
            }
        }
    }
}

fn read_snapshot(host: &OwnedHostApi, result_area_bytes: usize) -> Result<ResultSnapshot, String> {
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

    let mut area = vec![0u8; result_area_bytes.clamp(64, 4096)];
    read_exact(host, global + RESULT_AREA_OFFSET, &mut area, "result_area")?;
    let area_hash = hash_bytes(&area);

    Ok(ResultSnapshot {
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        reward_mode: read_u8(host, global + REWARD_MODE_OFFSET, "reward_mode")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        context: read_u32(host, global + CONTEXT_OFFSET, "context")?,
        work_flag: read_u32(host, global + RESULT_WORK_FLAG_OFFSET, "result_work_flag")?,
        area_hash,
        area,
    })
}

fn log_snapshot(
    host: &OwnedHostApi,
    previous: Option<&ResultSnapshot>,
    snapshot: &ResultSnapshot,
    max_changed_words: usize,
) {
    let changes = previous
        .map(|previous| changed_words(&previous.area, &snapshot.area, max_changed_words))
        .unwrap_or_else(|| non_zero_words(&snapshot.area, max_changed_words));
    let reason = previous.map_or("initial", |_| "changed");
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "result_probe {reason} mission_id={} difficulty={} mode_type={} reward_mode={} context=0x{:x} work_flag=0x{:x} area_hash=0x{:016x} global=0x{:x} words={changes}",
            snapshot.mission_id,
            snapshot.difficulty,
            snapshot.mode_type,
            snapshot.reward_mode,
            snapshot.context,
            snapshot.work_flag,
            snapshot.area_hash,
            snapshot.global,
        ),
    );
}

fn changed_words(previous: &[u8], current: &[u8], limit: usize) -> String {
    let mut changes = Vec::new();
    for (idx, (before, after)) in previous
        .chunks_exact(4)
        .zip(current.chunks_exact(4))
        .enumerate()
    {
        let before = u32::from_le_bytes([before[0], before[1], before[2], before[3]]);
        let after = u32::from_le_bytes([after[0], after[1], after[2], after[3]]);
        if before != after {
            changes.push(format!("+0x{:03x}:{}->{}", idx * 4, before, after));
            if changes.len() >= limit {
                break;
            }
        }
    }
    if changes.is_empty() {
        "none".to_string()
    } else {
        changes.join(",")
    }
}

fn non_zero_words(bytes: &[u8], limit: usize) -> String {
    let mut words = Vec::new();
    for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if value != 0 {
            words.push(format!("+0x{:03x}:{}", idx * 4, value));
            if words.len() >= limit {
                break;
            }
        }
    }
    if words.is_empty() {
        "none".to_string()
    } else {
        words.join(",")
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
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
    fn changed_words_reports_offsets_and_values() {
        let previous = [1, 0, 0, 0, 2, 0, 0, 0];
        let current = [1, 0, 0, 0, 5, 0, 0, 0];

        assert_eq!(changed_words(&previous, &current, 4), "+0x004:2->5");
    }

    #[test]
    fn non_zero_words_reports_initial_values() {
        let bytes = [0, 0, 0, 0, 7, 0, 0, 0, 9, 0, 0, 0];

        assert_eq!(non_zero_words(&bytes, 1), "+0x004:7");
    }

    #[test]
    fn hash_changes_when_bytes_change() {
        assert_ne!(hash_bytes(&[1, 2, 3]), hash_bytes(&[1, 2, 4]));
    }
}
