use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_exact, read_u16, read_u32, read_u8, read_usize};

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
pub(super) struct ResultSnapshot {
    pub(super) global: usize,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) reward_mode: u8,
    pub(super) difficulty: u8,
    pub(super) context: u32,
    pub(super) work_flag: u32,
    pub(super) area_hash: u64,
    pub(super) area: Vec<u8>,
}

pub(super) fn read(
    host: &OwnedHostApi,
    result_area_bytes: usize,
) -> Result<ResultSnapshot, String> {
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

    Ok(ResultSnapshot {
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        reward_mode: read_u8(host, global + REWARD_MODE_OFFSET, "reward_mode")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        context: read_u32(host, global + CONTEXT_OFFSET, "context")?,
        work_flag: read_u32(host, global + RESULT_WORK_FLAG_OFFSET, "result_work_flag")?,
        area_hash: hash_bytes(&area),
        area,
    })
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_changes_when_bytes_change() {
        assert_ne!(hash_bytes(&[1, 2, 3]), hash_bytes(&[1, 2, 4]));
    }
}
