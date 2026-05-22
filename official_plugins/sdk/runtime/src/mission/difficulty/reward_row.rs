use plugin_sdk::OwnedHostApi;

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
    direct_u32: [(usize, u32); 4],
    arrays_u16: [(usize, [u16; 4]); 10],
    bytes_39c: [u8; 4],
}

impl RewardRowDump {
    pub(crate) fn format_log(&self) -> String {
        let mut message = format!(
            "reward_row index={} fixed20=0x{:x} fixed28=0x{:x}",
            self.index, self.fixed20, self.fixed28
        );

        message.push_str(" u32=");
        for (idx, (offset, value)) in self.direct_u32.iter().enumerate() {
            if idx != 0 {
                message.push(',');
            }
            message.push_str(&format!("0x{offset:x}:{value}"));
        }

        message.push_str(" u16x4=");
        for (idx, (offset, values)) in self.arrays_u16.iter().enumerate() {
            if idx != 0 {
                message.push(',');
            }
            message.push_str(&format!(
                "0x{offset:x}:[{},{},{},{}]",
                values[0], values[1], values[2], values[3]
            ));
        }

        message.push_str(&format!(
            " bytes_39c=[{},{},{},{}]",
            self.bytes_39c[0], self.bytes_39c[1], self.bytes_39c[2], self.bytes_39c[3]
        ));
        message
    }
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
    fn formats_compact_reward_row_dump() {
        let dump = RewardRowDump {
            index: 42,
            fixed20: 0x1000,
            fixed28: 0x2000,
            direct_u32: [(0x334, 10), (0x33c, 20), (0x340, 30), (0x348, 40)],
            arrays_u16: [
                (0x34c, [1, 2, 3, 4]),
                (0x354, [5, 6, 7, 8]),
                (0x35c, [9, 10, 11, 12]),
                (0x364, [13, 14, 15, 16]),
                (0x36c, [17, 18, 19, 20]),
                (0x374, [21, 22, 23, 24]),
                (0x37c, [25, 26, 27, 28]),
                (0x384, [29, 30, 31, 32]),
                (0x38c, [33, 34, 35, 36]),
                (0x394, [37, 38, 39, 40]),
            ],
            bytes_39c: [4, 3, 2, 1],
        };

        let log = dump.format_log();
        assert!(log.contains("reward_row index=42"));
        assert!(log.contains("0x334:10"));
        assert!(log.contains("0x394:[37,38,39,40]"));
        assert!(log.contains("bytes_39c=[4,3,2,1]"));
    }
}
