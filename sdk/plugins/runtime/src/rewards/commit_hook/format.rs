use std::slice;

use serde::Serialize;

use crate::runtime::core::rewards::RewardCommitEvent;

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

pub(super) fn reward_event_log(call: usize, event: &RewardCommitEvent) -> String {
    let berry = event
        .rewards
        .berry
        .map(|reward| reward.amount.to_string())
        .unwrap_or_else(|| "none".to_string());

    format!("reward_event call={call} rank={} berry={berry}", event.rank)
}

pub(super) fn reward_event_payload(event: &RewardCommitEvent) -> RewardEventPayload {
    RewardEventPayload {
        schema: "sdk.runtime.rewards.event.v1",
        rank: event.rank.to_string(),
        berry: event.rewards.berry.map(|reward| reward.amount),
    }
}

#[derive(Debug, Serialize)]
pub(super) struct RewardEventPayload {
    schema: &'static str,
    rank: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    berry: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::core::{rank::RankValue, rewards::RewardState};

    #[test]
    fn reward_event_log_is_compact_and_stable() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(321));

        assert_eq!(
            reward_event_log(7, &event),
            "reward_event call=7 rank=S+ berry=321"
        );
    }

    #[test]
    fn reward_event_payload_is_serializable() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(321));

        let json = serde_json::to_string(&reward_event_payload(&event)).expect("json");

        assert!(json.contains(r#""rank":"S+""#));
        assert!(json.contains(r#""berry":321"#));
    }
}
