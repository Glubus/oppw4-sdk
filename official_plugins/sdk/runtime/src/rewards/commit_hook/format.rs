use std::slice;

use super::REWARD_SLOT_COUNT;

pub(super) fn reward_log(
    call: usize,
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
) -> String {
    let slots = unsafe { slice::from_raw_parts(reward_out.cast_const(), REWARD_SLOT_COUNT) };
    format!(
        "reward_probe call={call} out=0x{:x} param2={} param3={} param4={} param5={} param6={} slots=[{}, {}, {}, {}, {}, {}, {}, {}]",
        reward_out as usize,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
        slots[0],
        slots[1],
        slots[2],
        slots[3],
        slots[4],
        slots[5],
        slots[6],
        slots[7],
    )
}
