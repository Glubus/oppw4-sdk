use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_u16, read_u32, read_u8, read_usize};

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
const SAVE_PTR_OFFSET: usize = 0x10;
const GLOBAL_PTR_OFFSET: usize = 0x28;

const ACTIVE_PLAYER_OFFSET: usize = 0x31;
const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const DIFFICULTY_OFFSET: usize = 0x1d756;

const PLAYER_STRIDE: usize = 0xb90;
pub(super) const PLAYER_RESULT_STAT_OFFSETS: [usize; 12] = [
    0x8fc, 0x900, 0x904, 0x908, 0x90c, 0x910, 0x914, 0x918, 0x91c, 0x920, 0x924, 0x928,
];
pub(super) const SAVE_RESULT_TOTAL_OFFSETS: [usize; 12] = [
    0x764, 0x768, 0x76c, 0x770, 0x774, 0x778, 0x77c, 0x780, 0x784, 0x788, 0x78c, 0x790,
];
pub(super) const SAVE_MODE_TOTAL_OFFSETS: [usize; 3] = [0x794, 0x798, 0x79c];
pub(super) const SOUL_STATE_OFFSET: usize = 0xfe6c;
pub(super) const SOUL_STATE_WORDS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlayerResultSnapshot {
    pub(super) global: usize,
    pub(super) save: usize,
    pub(super) active_player: u8,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) difficulty: u8,
    pub(super) player_stats: [[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4],
    pub(super) save_totals: [u32; SAVE_RESULT_TOTAL_OFFSETS.len()],
    pub(super) save_mode_totals: [u32; SAVE_MODE_TOTAL_OFFSETS.len()],
    pub(super) soul_state: [u32; SOUL_STATE_WORDS],
}

pub(super) fn read(host: &OwnedHostApi) -> Result<PlayerResultSnapshot, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, base + GLOBAL_ROOT_RVA, "global_root")?;
    let first = read_usize(host, root + GLOBAL_PTR_FIRST_OFFSET, "global_root+0x18")?;
    let save = read_usize(host, first + SAVE_PTR_OFFSET, "save_state")?;
    let global = read_usize(host, first + GLOBAL_PTR_OFFSET, "global_state")?;
    if save == 0 {
        return Err("save_state is null".to_string());
    }
    if global == 0 {
        return Err("global_state is null".to_string());
    }

    Ok(PlayerResultSnapshot {
        global,
        save,
        active_player: read_u8(host, global + ACTIVE_PLAYER_OFFSET, "active_player")?,
        mission_id: read_u16(host, global + MISSION_ID_OFFSET, "mission_id")?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        player_stats: read_player_stats(host, global)?,
        save_totals: read_u32_offsets(host, save, &SAVE_RESULT_TOTAL_OFFSETS)?,
        save_mode_totals: read_u32_offsets(host, save, &SAVE_MODE_TOTAL_OFFSETS)?,
        soul_state: read_soul_state(host, save)?,
    })
}

fn read_player_stats(
    host: &OwnedHostApi,
    global: usize,
) -> Result<[[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4], String> {
    let mut players = [[0u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4];
    for (player_index, stats) in players.iter_mut().enumerate() {
        let player_base = global + player_index * PLAYER_STRIDE;
        *stats = read_u32_offsets(host, player_base, &PLAYER_RESULT_STAT_OFFSETS)?;
    }
    Ok(players)
}

fn read_soul_state(host: &OwnedHostApi, save: usize) -> Result<[u32; SOUL_STATE_WORDS], String> {
    let mut values = [0u32; SOUL_STATE_WORDS];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(
            host,
            save + SOUL_STATE_OFFSET + index * size_of::<u32>(),
            "soul_state",
        )?;
    }
    Ok(values)
}

fn read_u32_offsets<const N: usize>(
    host: &OwnedHostApi,
    base: usize,
    offsets: &[usize; N],
) -> Result<[u32; N], String> {
    let mut values = [0u32; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(host, base + offsets[index], "u32_offset")?;
    }
    Ok(values)
}
