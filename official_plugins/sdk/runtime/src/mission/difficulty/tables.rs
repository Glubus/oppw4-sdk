use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_u16, read_u32, read_usize};

use super::DifficultyId;

const FIXED_ROOT_RVA: usize = 0x1eba738;
const FIXED_OWNER_OFFSET: usize = 0x18;
const FIXED_REWARD_ROWS_OFFSET: usize = 0x20;
const FIXED_REWARD_INDEX_OFFSET: usize = 0x28;

#[cfg_attr(not(test), allow(dead_code))]
const BASE_MISSION_LIMIT: u16 = 0x00f9;
const BASE_REWARD_INDEX_OFFSET: usize = 0x00a8;
const BASE_REWARD_INDEX_STRIDE: usize = 0x006e;
const REWARD_ROW_STRIDE: usize = 0x006c;

const FIELD_334_OFFSET: usize = 0x334;
const FIELD_33C_OFFSET: usize = 0x33c;
const FIELD_340_OFFSET: usize = 0x340;
const FIELD_348_OFFSET: usize = 0x348;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MissionDifficulty {
    pub(crate) mission_id: u16,
    pub(crate) difficulty: DifficultyId,
}

impl MissionDifficulty {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn new(mission_id: u16, difficulty: DifficultyId) -> Result<Self, String> {
        if mission_id > BASE_MISSION_LIMIT {
            return Err(format!(
                "mission id {mission_id} is outside vanilla mission table"
            ));
        }
        Ok(Self {
            mission_id,
            difficulty,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct DifficultyRowFields {
    pub(crate) field_334: u32,
    pub(crate) actor_stat_33c: u32,
    pub(crate) actor_stat_340: u32,
    pub(crate) field_348: u32,
}

#[allow(dead_code)]
pub(crate) fn reward_row_index(host: &OwnedHostApi, key: MissionDifficulty) -> Result<u16, String> {
    let refs = FixedDifficultyTables::read(host)?;
    read_u16(
        host,
        refs.reward_indexes + reward_index_offset(key),
        "difficulty_reward_row_index",
    )
}

#[allow(dead_code)]
pub(crate) fn read_row_fields(
    host: &OwnedHostApi,
    row_index: u16,
) -> Result<DifficultyRowFields, String> {
    let refs = FixedDifficultyTables::read(host)?;
    let row = refs.row_base(row_index);
    Ok(DifficultyRowFields {
        field_334: read_u32(host, row + FIELD_334_OFFSET, "difficulty_row_0x334")?,
        actor_stat_33c: read_u32(host, row + FIELD_33C_OFFSET, "difficulty_row_0x33c")?,
        actor_stat_340: read_u32(host, row + FIELD_340_OFFSET, "difficulty_row_0x340")?,
        field_348: read_u32(host, row + FIELD_348_OFFSET, "difficulty_row_0x348")?,
    })
}

fn reward_index_offset(key: MissionDifficulty) -> usize {
    BASE_REWARD_INDEX_OFFSET
        + (usize::from(key.mission_id) * BASE_REWARD_INDEX_STRIDE
            + usize::from(key.difficulty.as_u8()))
            * size_of::<u16>()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FixedDifficultyTables {
    reward_rows: usize,
    reward_indexes: usize,
}

impl FixedDifficultyTables {
    fn read(host: &OwnedHostApi) -> Result<Self, String> {
        let module_base = host
            .memory()
            .module_base()
            .map_err(|error| format!("module_base failed: {error}"))?;
        if module_base == 0 {
            return Err("module base is null".to_string());
        }

        let root = read_usize(host, module_base + FIXED_ROOT_RVA, "fixed_root")?;
        if root == 0 {
            return Err("fixed root is null".to_string());
        }

        let owner = read_usize(host, root + FIXED_OWNER_OFFSET, "fixed_root+0x18")?;
        if owner == 0 {
            return Err("fixed owner is null".to_string());
        }

        let reward_rows = read_usize(
            host,
            owner + FIXED_REWARD_ROWS_OFFSET,
            "difficulty_reward_rows",
        )?;
        let reward_indexes = read_usize(
            host,
            owner + FIXED_REWARD_INDEX_OFFSET,
            "difficulty_reward_indexes",
        )?;
        if reward_rows == 0 || reward_indexes == 0 {
            return Err("fixed difficulty table pointer is null".to_string());
        }

        Ok(Self {
            reward_rows,
            reward_indexes,
        })
    }

    fn row_base(self, row_index: u16) -> usize {
        self.reward_rows + usize::from(row_index) * REWARD_ROW_STRIDE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missions_outside_base_table() {
        assert!(MissionDifficulty::new(0x00fa, DifficultyId::Easy).is_err());
    }

    #[test]
    fn computes_vanilla_reward_index_offsets() {
        let key = MissionDifficulty::new(35, DifficultyId::Easy).unwrap();
        assert_eq!(reward_index_offset(key), 0x00a8 + (35 * 0x6e) * 2);

        let key = MissionDifficulty::new(35, DifficultyId::SuperHard).unwrap();
        assert_eq!(reward_index_offset(key), 0x00a8 + (35 * 0x6e + 3) * 2);
    }
}
