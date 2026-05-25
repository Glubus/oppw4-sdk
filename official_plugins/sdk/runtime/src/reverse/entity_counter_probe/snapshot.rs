use plugin_sdk::OwnedHostApi;

use crate::{
    config::EntityCounterProbeConfig,
    runtime::reader::{read_exact, read_u16, read_u8, read_usize},
};

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_OWNER_OFFSET: usize = 0x18;
const GLOBAL_STATE_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CounterSnapshot {
    pub(super) global: usize,
    pub(super) mission_id: u16,
    pub(super) difficulty: u8,
    pub(super) mode_type: u8,
    pub(super) bytes: Vec<u8>,
    pub(super) changes: Vec<CounterChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CounterChange {
    offset: usize,
    width: CounterWidth,
    previous: u32,
    current: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CounterWidth {
    U16,
    U32,
}

pub(super) fn read(
    host: &OwnedHostApi,
    config: &EntityCounterProbeConfig,
    previous: Option<&Vec<u8>>,
) -> Result<CounterSnapshot, String> {
    let module_base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if module_base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, module_base + GLOBAL_ROOT_RVA, "global_root")?;
    let owner = read_usize(host, root + GLOBAL_OWNER_OFFSET, "global_root+0x18")?;
    let global = read_usize(host, owner + GLOBAL_STATE_OFFSET, "global_state")?;
    if global == 0 {
        return Err("global_state is null".to_string());
    }

    let scan_bytes = config.scan_bytes.clamp(4096, 0x100000);
    let mut bytes = vec![0; scan_bytes];
    read_exact(host, global, &mut bytes, "entity_counter_scan")?;

    let changes = previous
        .filter(|old| old.len() == bytes.len())
        .map(|old| diff_counter_candidates(old, &bytes, config.max_value, config.max_changes))
        .unwrap_or_default();

    Ok(CounterSnapshot {
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        bytes,
        changes,
    })
}

impl CounterSnapshot {
    pub(super) fn format_log(&self) -> String {
        format!(
            "entity_counter_probe mission_id={} difficulty={} mode_type={} global=0x{:x} changes={}",
            self.mission_id,
            self.difficulty,
            self.mode_type,
            self.global,
            format_changes(&self.changes),
        )
    }
}

fn diff_counter_candidates(
    previous: &[u8],
    current: &[u8],
    max_value: u32,
    max_changes: usize,
) -> Vec<CounterChange> {
    let mut changes = Vec::new();
    collect_u32_changes(previous, current, max_value, max_changes, &mut changes);
    collect_u16_changes(previous, current, max_value, max_changes, &mut changes);
    changes
}

fn collect_u16_changes(
    previous: &[u8],
    current: &[u8],
    max_value: u32,
    max_changes: usize,
    changes: &mut Vec<CounterChange>,
) {
    for offset in (0..current.len().saturating_sub(1)).step_by(2) {
        if changes
            .iter()
            .any(|change| change.width == CounterWidth::U32 && change.offset == offset)
        {
            continue;
        }
        let old = u32::from(u16::from_le_bytes([previous[offset], previous[offset + 1]]));
        let new = u32::from(u16::from_le_bytes([current[offset], current[offset + 1]]));
        if is_counter_change(old, new, max_value) {
            changes.push(CounterChange {
                offset,
                width: CounterWidth::U16,
                previous: old,
                current: new,
            });
        }
        if changes.len() >= max_changes {
            return;
        }
    }
}

fn collect_u32_changes(
    previous: &[u8],
    current: &[u8],
    max_value: u32,
    max_changes: usize,
    changes: &mut Vec<CounterChange>,
) {
    for offset in (0..current.len().saturating_sub(3)).step_by(4) {
        let old = u32::from_le_bytes([
            previous[offset],
            previous[offset + 1],
            previous[offset + 2],
            previous[offset + 3],
        ]);
        let new = u32::from_le_bytes([
            current[offset],
            current[offset + 1],
            current[offset + 2],
            current[offset + 3],
        ]);
        if is_counter_change(old, new, max_value) {
            changes.push(CounterChange {
                offset,
                width: CounterWidth::U32,
                previous: old,
                current: new,
            });
        }
        if changes.len() >= max_changes {
            return;
        }
    }
}

fn is_counter_change(previous: u32, current: u32, max_value: u32) -> bool {
    previous != current && previous <= max_value && current <= max_value
}

fn format_changes(changes: &[CounterChange]) -> String {
    changes
        .iter()
        .map(|change| {
            format!(
                "+0x{:x}:{}:{}->{}",
                change.offset,
                change.width.format_log(),
                change.previous,
                change.current,
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

impl CounterWidth {
    fn format_log(self) -> &'static str {
        match self {
            Self::U16 => "u16",
            Self::U32 => "u32",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_small_counter_changes() {
        let mut previous = vec![0; 16];
        let mut current = previous.clone();
        previous[2..4].copy_from_slice(&12u16.to_le_bytes());
        current[2..4].copy_from_slice(&18u16.to_le_bytes());
        previous[8..12].copy_from_slice(&100u32.to_le_bytes());
        current[8..12].copy_from_slice(&110u32.to_le_bytes());

        let changes = diff_counter_candidates(&previous, &current, 5000, 8);

        assert_eq!(changes.len(), 2);
        assert!(changes
            .iter()
            .any(|change| change.offset == 2 && change.previous == 12 && change.current == 18));
        assert!(changes
            .iter()
            .any(|change| change.offset == 8 && change.previous == 100 && change.current == 110));
    }

    #[test]
    fn ignores_large_pointer_like_changes() {
        let mut previous = vec![0; 8];
        let mut current = previous.clone();
        previous[0..4].copy_from_slice(&10_000_000u32.to_le_bytes());
        current[0..4].copy_from_slice(&10_000_001u32.to_le_bytes());

        let changes = diff_counter_candidates(&previous, &current, 5000, 8);

        assert!(changes.is_empty());
    }
}
