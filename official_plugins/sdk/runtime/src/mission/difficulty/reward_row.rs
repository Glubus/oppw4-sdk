use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_exact, read_u16, read_u32, read_usize};

mod format;

const FIXED_DATA_ROOT_RVA: usize = 0x1eba738;
const FIXED_PTR_FIRST_OFFSET: usize = 0x18;
const FIXED_REWARD_ROWS_OFFSET: usize = 0x20;
const FIXED_REWARD_INDEX_OFFSET: usize = 0x28;

const BASE_MISSION_LIMIT: u16 = 0x00f9;
const VANILLA_DIFFICULTY_LIMIT: u8 = 3;
const BASE_REWARD_INDEX_OFFSET: usize = 0x00a8;
const BASE_REWARD_INDEX_STRIDE: usize = 0x006e;
const REWARD_ROW_STRIDE: usize = 0x006c;

const DIRECT_U32_FIELDS: [usize; 4] = [0x334, 0x33c, 0x340, 0x348];
const ARRAY_U16_BASES: [usize; 10] = [
    0x34c, 0x354, 0x35c, 0x364, 0x36c, 0x374, 0x37c, 0x384, 0x38c, 0x394,
];
const BYTE_FIELD_39C: usize = 0x39c;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RewardRowDump {
    pub(crate) index: u16,
    pub(crate) fixed20: usize,
    pub(crate) fixed28: usize,
    pub(super) direct_u32: [(usize, u32); 4],
    pub(super) arrays_u16: [(usize, [u16; 4]); 10],
    pub(super) bytes_39c: [u8; 4],
}

pub(crate) fn read_reward_row_dump(
    host: &OwnedHostApi,
    module_base: usize,
    mission_id: u16,
    difficulty: u8,
) -> Result<Option<RewardRowDump>, String> {
    if mission_id > BASE_MISSION_LIMIT || difficulty > VANILLA_DIFFICULTY_LIMIT {
        return Ok(None);
    }

    let fixed_root = read_usize(host, module_base + FIXED_DATA_ROOT_RVA, "fixed_data_root")?;
    let fixed_first = read_usize(
        host,
        fixed_root + FIXED_PTR_FIRST_OFFSET,
        "fixed_data_root+0x18",
    )?;
    let fixed20 = read_usize(
        host,
        fixed_first + FIXED_REWARD_ROWS_OFFSET,
        "fixed_data_rows",
    )?;
    let fixed28 = read_usize(
        host,
        fixed_first + FIXED_REWARD_INDEX_OFFSET,
        "fixed_data_indexes",
    )?;
    if fixed20 == 0 || fixed28 == 0 {
        return Err("fixed reward data pointer is null".to_string());
    }

    let index_offset = BASE_REWARD_INDEX_OFFSET
        + (usize::from(mission_id) * BASE_REWARD_INDEX_STRIDE + usize::from(difficulty)) * 2;
    let index = read_u16(host, fixed28 + index_offset, "base_reward_row_index")?;
    let row_base = fixed20 + usize::from(index) * REWARD_ROW_STRIDE;

    let mut direct_u32 = [(0usize, 0u32); 4];
    for (slot, offset) in direct_u32.iter_mut().zip(DIRECT_U32_FIELDS) {
        *slot = (offset, read_u32(host, row_base + offset, "reward_row_u32")?);
    }

    let mut arrays_u16 = [(0usize, [0u16; 4]); 10];
    for (slot, offset) in arrays_u16.iter_mut().zip(ARRAY_U16_BASES) {
        let mut values = [0u16; 4];
        for (idx, value) in values.iter_mut().enumerate() {
            *value = read_u16(host, row_base + offset + idx * 2, "reward_row_u16x4")?;
        }
        *slot = (offset, values);
    }

    let mut bytes_39c = [0u8; 4];
    read_exact(
        host,
        row_base + BYTE_FIELD_39C,
        &mut bytes_39c,
        "reward_row_39c",
    )?;

    Ok(Some(RewardRowDump {
        index,
        fixed20,
        fixed28,
        direct_u32,
        arrays_u16,
        bytes_39c,
    }))
}
