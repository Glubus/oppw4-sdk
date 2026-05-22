use std::slice;

use super::{RewardCommitSnapshot, REWARD_SLOT_COUNT};

pub(super) fn snapshot(
    call: usize,
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
) -> RewardCommitSnapshot {
    let slots = unsafe { slice::from_raw_parts(reward_out.cast_const(), REWARD_SLOT_COUNT) };
    RewardCommitSnapshot {
        call,
        reward_out: reward_out as usize,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
        slots: slots.try_into().unwrap_or([0; REWARD_SLOT_COUNT]),
    }
}

pub(super) fn reward_log(snapshot: &RewardCommitSnapshot) -> String {
    format!(
        "reward_probe call={call} out=0x{:x} param2={} param3={} param4={} param5={} param6={} slots=[{}, {}, {}, {}, {}, {}, {}, {}]",
        snapshot.reward_out,
        snapshot.reward_param,
        snapshot.mission_or_reward,
        snapshot.rank_or_mode,
        snapshot.bonus_a,
        snapshot.bonus_b,
        snapshot.slots[0],
        snapshot.slots[1],
        snapshot.slots[2],
        snapshot.slots[3],
        snapshot.slots[4],
        snapshot.slots[5],
        snapshot.slots[6],
        snapshot.slots[7],
        call = snapshot.call,
    )
}
