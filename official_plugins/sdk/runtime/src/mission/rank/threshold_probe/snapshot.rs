use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::runtime::reader::{read_u16_block, read_u8, read_usize};

use super::layout::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RankThresholdSnapshot {
    pub(super) global: usize,
    pub(super) fixed_rank_table: usize,
    pub(super) active_player: u8,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) difficulty: u8,
    pub(super) slots: [RankSlot; RESULT_RANK_SLOT_COUNT],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RankSlot {
    pub(super) slot_index: usize,
    pub(super) rank_row_id: u16,
    pub(super) raw_words: [u16; RESULT_RANK_SLOT_WORDS],
    pub(super) fixed_row_words: Option<[u16; FIXED_RANK_ROW_WORDS]>,
    pub(super) condition_row_id: Option<u16>,
    pub(super) condition_row_words: Option<[u16; FIXED_CONDITION_ROW_WORDS]>,
}

#[derive(Debug, Serialize)]
pub(super) struct RankThresholdSignal {
    pub(super) global: usize,
    pub(super) fixed_rank_table: usize,
    pub(super) active_player: u8,
    pub(super) mission_id: u16,
    pub(super) mode_type: u8,
    pub(super) difficulty: u8,
    pub(super) slots: Vec<RankSlotSignal>,
}

#[derive(Debug, Serialize)]
pub(super) struct RankSlotSignal {
    pub(super) slot_index: usize,
    pub(super) rank_row_id: u16,
    pub(super) raw_words: Vec<u16>,
    pub(super) fixed_row_words: Option<Vec<u16>>,
    pub(super) condition_row_id: Option<u16>,
    pub(super) condition_row_words: Option<Vec<u16>>,
}

impl RankThresholdSnapshot {
    pub(super) fn signal_payload(&self) -> RankThresholdSignal {
        RankThresholdSignal {
            global: self.global,
            fixed_rank_table: self.fixed_rank_table,
            active_player: self.active_player,
            mission_id: self.mission_id,
            mode_type: self.mode_type,
            difficulty: self.difficulty,
            slots: self.slots.iter().map(RankSlot::signal_payload).collect(),
        }
    }
}

impl RankSlot {
    fn signal_payload(&self) -> RankSlotSignal {
        RankSlotSignal {
            slot_index: self.slot_index,
            rank_row_id: self.rank_row_id,
            raw_words: self.raw_words.to_vec(),
            fixed_row_words: self.fixed_row_words.map(|words| words.to_vec()),
            condition_row_id: self.condition_row_id,
            condition_row_words: self.condition_row_words.map(|words| words.to_vec()),
        }
    }
}

pub(super) fn read(host: &OwnedHostApi) -> Result<RankThresholdSnapshot, String> {
    let base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if base == 0 {
        return Err("module base is null".to_string());
    }

    let global = read_global_state(host, base)?;
    let fixed_rank_table = read_fixed_rank_table(host, base)?;
    if fixed_rank_table == 0 {
        return Err("fixed rank table is null".to_string());
    }

    Ok(RankThresholdSnapshot {
        global,
        fixed_rank_table,
        active_player: read_u8(host, global + ACTIVE_PLAYER_OFFSET, "active_player")?,
        mission_id: crate::runtime::reader::read_u16(
            host,
            global + MISSION_ID_OFFSET,
            "mission_id",
        )?,
        mode_type: read_u8(host, global + MODE_TYPE_OFFSET, "mode_type")?,
        difficulty: read_u8(host, global + DIFFICULTY_OFFSET, "difficulty")?,
        slots: read_slots(host, global, fixed_rank_table)?,
    })
}

fn read_global_state(host: &OwnedHostApi, base: usize) -> Result<usize, String> {
    let root = read_usize(host, base + GLOBAL_ROOT_RVA, "global_root")?;
    let first = read_usize(host, root + GLOBAL_PTR_FIRST_OFFSET, "global_root+0x18")?;
    let global = read_usize(host, first + GLOBAL_PTR_SECOND_OFFSET, "global_state")?;
    if global == 0 {
        return Err("global_state is null".to_string());
    }
    Ok(global)
}

fn read_fixed_rank_table(host: &OwnedHostApi, base: usize) -> Result<usize, String> {
    let root = read_usize(host, base + FIXED_ROOT_RVA, "fixed_root")?;
    let owner = read_usize(host, root + FIXED_TABLE_OWNER_OFFSET, "fixed_root+0x18")?;
    read_usize(
        host,
        owner + FIXED_RANK_TABLE_OFFSET,
        "fixed_rank_table_owner+0x8",
    )
}

fn read_slots(
    host: &OwnedHostApi,
    global: usize,
    fixed_rank_table: usize,
) -> Result<[RankSlot; RESULT_RANK_SLOT_COUNT], String> {
    let mut slots = Vec::with_capacity(RESULT_RANK_SLOT_COUNT);
    for slot_index in 0..RESULT_RANK_SLOT_COUNT {
        slots.push(read_slot(host, global, fixed_rank_table, slot_index)?);
    }

    slots
        .try_into()
        .map_err(|_| "rank slot count mismatch".to_string())
}

fn read_slot(
    host: &OwnedHostApi,
    global: usize,
    fixed_rank_table: usize,
    slot_index: usize,
) -> Result<RankSlot, String> {
    let slot_address = global + RESULT_RANK_AREA_OFFSET + slot_index * RESULT_RANK_SLOT_STRIDE;
    let raw_words = read_u16_block::<RESULT_RANK_SLOT_WORDS>(host, slot_address, "rank_slot")?;
    let rank_row_id = raw_words[0];
    let fixed_row_words = read_fixed_rank_row(host, fixed_rank_table, rank_row_id).ok();
    let condition_row_id = fixed_row_words.and_then(condition_row_id);
    let condition_row_words = condition_row_id.and_then(|row_id| {
        read_condition_row(host, fixed_rank_table, row_id)
            .ok()
            .flatten()
    });

    Ok(RankSlot {
        slot_index,
        rank_row_id,
        raw_words,
        fixed_row_words,
        condition_row_id,
        condition_row_words,
    })
}

fn read_fixed_rank_row(
    host: &OwnedHostApi,
    fixed_rank_table: usize,
    rank_row_id: u16,
) -> Result<[u16; FIXED_RANK_ROW_WORDS], String> {
    let address = fixed_rank_table + usize::from(rank_row_id) * FIXED_RANK_ROW_STRIDE;
    read_u16_block::<FIXED_RANK_ROW_WORDS>(host, address, "fixed_rank_row")
}

fn condition_row_id(row: [u16; FIXED_RANK_ROW_WORDS]) -> Option<u16> {
    let value = row[0x16 / 2];
    if value == u16::MAX {
        None
    } else {
        Some(value)
    }
}

fn read_condition_row(
    host: &OwnedHostApi,
    fixed_rank_table: usize,
    row_id: u16,
) -> Result<Option<[u16; FIXED_CONDITION_ROW_WORDS]>, String> {
    if row_id >= KNOWN_CONDITION_ROW_LIMIT {
        return Ok(None);
    }
    let address = fixed_rank_table
        + FIXED_CONDITION_TABLE_OFFSET
        + usize::from(row_id) * FIXED_CONDITION_ROW_STRIDE;
    read_u16_block::<FIXED_CONDITION_ROW_WORDS>(host, address, "rank_condition_row").map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_condition_row_id_from_fixed_row() {
        let mut row = [0u16; FIXED_RANK_ROW_WORDS];
        row[0x16 / 2] = 12;
        assert_eq!(condition_row_id(row), Some(12));

        row[0x16 / 2] = u16::MAX;
        assert_eq!(condition_row_id(row), None);
    }
}
