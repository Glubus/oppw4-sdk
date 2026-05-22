use super::{
    hash::calculate_hash,
    reader::{read_crew_unlocks, read_events, read_u32, read_u32_block},
};

pub(super) const U32_BLOCK_WORDS: usize = 14;
pub(super) const CREW_POINT_BLOCK_OFFSET: usize = 0x498;
pub(super) const CREW_POINT_BLOCK_WORDS: usize = 14;
pub(super) const CREW_UNLOCK_WORDS: usize = 2;
pub(super) const EVENT_WORDS: usize = 5;

const MAX_CREW_UNLOCKS: usize = 32;

#[derive(Debug)]
pub(super) struct ResultStateSnapshot {
    pub(super) address: usize,
    pub(super) state: u32,
    pub(super) unlock_flags: u32,
    pub(super) mission_id: u32,
    pub(super) difficulty_or_rank: [u32; 5],
    pub(super) result_copy: [u32; U32_BLOCK_WORDS],
    pub(super) crew_points: [u32; CREW_POINT_BLOCK_WORDS],
    pub(super) source_rewards: [u32; U32_BLOCK_WORDS],
    pub(super) soul_counter: u32,
    pub(super) character_rewards: [u32; 5],
    pub(super) character_totals: [u32; 5],
    pub(super) crew_unlock_count: u32,
    pub(super) crew_unlocks: Vec<[u32; CREW_UNLOCK_WORDS]>,
    pub(super) event_count: usize,
    pub(super) events: Vec<[u32; EVENT_WORDS]>,
    hash: u64,
}

impl ResultStateSnapshot {
    pub(super) unsafe fn read(result_state: *const u8, max_events: usize) -> Option<Self> {
        if result_state.is_null() {
            return None;
        }

        let crew_unlock_count = read_u32(result_state, 0x920);
        let crew_unlocks = read_crew_unlocks(
            result_state,
            crew_unlock_count.min(MAX_CREW_UNLOCKS as u32) as usize,
        );
        let events = read_events(result_state, max_events);

        let mut snapshot = Self {
            address: result_state as usize,
            state: read_u32(result_state, 0x10),
            unlock_flags: read_u32(result_state, 0x24),
            mission_id: read_u32(result_state, 0x40),
            difficulty_or_rank: read_u32_block::<5>(result_state, 0x28),
            result_copy: read_u32_block::<U32_BLOCK_WORDS>(result_state, 0x2ac),
            crew_points: read_u32_block::<CREW_POINT_BLOCK_WORDS>(
                result_state,
                CREW_POINT_BLOCK_OFFSET,
            ),
            source_rewards: read_u32_block::<U32_BLOCK_WORDS>(result_state, 0x7a0),
            soul_counter: read_u32(result_state, 0x2e8),
            character_rewards: read_u32_block::<5>(result_state, 0x2ec),
            character_totals: read_u32_block::<5>(result_state, 0x300),
            crew_unlock_count,
            crew_unlocks,
            event_count: events.len(),
            events,
            hash: 0,
        };
        snapshot.hash = calculate_hash(&snapshot);
        Some(snapshot)
    }

    pub(super) fn hash(&self) -> u64 {
        self.hash
    }
}
