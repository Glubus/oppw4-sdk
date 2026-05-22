use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_u16, read_u32, read_u8, read_usize};

use super::labels::{difficulty_label, mode_type_label};

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const CACHED_MISSION_RVA: usize = 0x1e5ec00;
const CACHED_DIFFICULTY_RVA: usize = 0x1e5ec04;

const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const RESULT_MODE_OFFSET: usize = 0x1d753;
const REWARD_MODE_OFFSET: usize = 0x1d754;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const SPECIAL_FLAG_OFFSET: usize = 0x1d762;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DifficultySnapshot {
    pub(super) module_base: usize,
    pub(super) global: usize,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) reward_mode: u8,
    pub(super) difficulty: u8,
    pub(super) special_flag: u8,
    pub(super) cached_mission: u32,
    pub(super) cached_difficulty: u32,
}

impl DifficultySnapshot {
    pub(super) fn format_log(self) -> String {
        format!(
            "difficulty_probe mission_id={} difficulty={}({}) mode_type={}({}) reward_mode={} special_flag={} cached_mission={} cached_difficulty={} global=0x{:x}",
            self.mission_id,
            self.difficulty,
            difficulty_label(self.difficulty),
            self.mode_type,
            mode_type_label(self.mode_type),
            self.reward_mode,
            self.special_flag,
            self.cached_mission,
            self.cached_difficulty,
            self.global,
        )
    }
}

pub(super) fn read(host: &OwnedHostApi) -> Result<DifficultySnapshot, String> {
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

    Ok(DifficultySnapshot {
        module_base: base,
        global,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + RESULT_MODE_OFFSET, "mode_type")?,
        reward_mode: read_u8(host, global + REWARD_MODE_OFFSET, "reward_mode")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        special_flag: read_u8(host, global + SPECIAL_FLAG_OFFSET, "special_flag")?,
        cached_mission: read_u32(host, base + CACHED_MISSION_RVA, "cached_mission")?,
        cached_difficulty: read_u32(host, base + CACHED_DIFFICULTY_RVA, "cached_difficulty")?,
    })
}
