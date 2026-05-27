use hooks::module_base;

use super::{
    FIXED_HELPER_TABLE_OFFSET, FIXED_ROOT_RVA, FIXED_SCORE_TABLE_OFFSET, FIXED_TABLE_OWNER_OFFSET,
    HOST,
};

pub(super) fn rank_row_offset(row: usize) -> Option<usize> {
    let fixed_helper_table = read_fixed_helper_table().ok()?;
    row.checked_sub(fixed_helper_table)
        .filter(|offset| *offset <= 0x10_0000)
}

pub(super) fn read_fixed_helper_table() -> Result<usize, String> {
    let base = module_base();
    let root = read_process_usize(base + FIXED_ROOT_RVA)?;
    let owner = read_process_usize(root + FIXED_TABLE_OWNER_OFFSET)?;
    read_process_usize(owner + FIXED_HELPER_TABLE_OFFSET)
}

pub(super) fn read_fixed_score_table() -> Result<usize, String> {
    let base = module_base();
    let root = read_process_usize(base + FIXED_ROOT_RVA)?;
    let owner = read_process_usize(root + FIXED_TABLE_OWNER_OFFSET)?;
    read_process_usize(owner + FIXED_SCORE_TABLE_OFFSET)
}

fn read_process_usize(address: usize) -> Result<usize, String> {
    let bytes = read_process_bytes::<8>(address)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

pub(super) fn read_process_i32(address: usize) -> Result<i32, String> {
    let bytes = read_process_bytes::<4>(address)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_process_bytes<const N: usize>(address: usize) -> Result<[u8; N], String> {
    let Some(host) = HOST.get() else {
        return Err("host unavailable".to_string());
    };
    let mut bytes = [0u8; N];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("read failed address=0x{address:x}: {error}"))?;
    Ok(bytes)
}
