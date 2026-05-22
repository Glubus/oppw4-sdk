use plugin_sdk::OwnedHostApi;

use crate::{
    config::ValueProbeConfig,
    runtime::reader::{read_exact, read_u16, read_u8, read_usize},
};

use super::scan::{scan_values, ValueHit};

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ValueSnapshot {
    pub(super) global: usize,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) difficulty: u8,
    pub(super) hits: Vec<ValueHit>,
}

pub(super) fn read(
    host: &OwnedHostApi,
    config: &ValueProbeConfig,
) -> Result<ValueSnapshot, String> {
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
