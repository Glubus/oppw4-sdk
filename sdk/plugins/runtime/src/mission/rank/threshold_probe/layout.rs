pub(super) const GLOBAL_ROOT_RVA: usize = 0x1eba750;
pub(super) const FIXED_ROOT_RVA: usize = 0x1eba738;

pub(super) const GLOBAL_PTR_FIRST_OFFSET: usize = 0x18;
pub(super) const GLOBAL_PTR_SECOND_OFFSET: usize = 0x28;
pub(super) const FIXED_TABLE_OWNER_OFFSET: usize = 0x18;
pub(super) const FIXED_RANK_TABLE_OFFSET: usize = 0x8;

pub(super) const ACTIVE_PLAYER_OFFSET: usize = 0x31;
pub(super) const MISSION_ID_OFFSET: usize = 0x1d750;
pub(super) const MODE_TYPE_OFFSET: usize = 0x1d753;
pub(super) const DIFFICULTY_OFFSET: usize = 0x1d756;
pub(super) const RESULT_RANK_AREA_OFFSET: usize = 0x1d9b0;
pub(super) const RESULT_RANK_SLOT_STRIDE: usize = 0x50;
pub(super) const RESULT_RANK_SLOT_COUNT: usize = 4;
pub(super) const RESULT_RANK_SLOT_WORDS: usize = RESULT_RANK_SLOT_STRIDE / 2;

pub(super) const FIXED_RANK_ROW_STRIDE: usize = 0x44;
pub(super) const FIXED_RANK_ROW_WORDS: usize = FIXED_RANK_ROW_STRIDE / 2;
pub(super) const FIXED_CONDITION_TABLE_OFFSET: usize = 0xc43c;
pub(super) const FIXED_CONDITION_ROW_STRIDE: usize = 0x34;
pub(super) const FIXED_CONDITION_ROW_WORDS: usize = FIXED_CONDITION_ROW_STRIDE / 2;
pub(super) const KNOWN_CONDITION_ROW_LIMIT: u16 = 0x68;
