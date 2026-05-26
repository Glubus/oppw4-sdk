use std::slice;

use serde::Serialize;

use crate::runtime::core::{
    events::RuntimeMutation,
    rewards::{MedalReward, RewardCommitEvent, RewardMutation, SoulReward},
};

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
    let mission = event
        .mission_id
        .map(|mission_id| mission_id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mode = event
        .difficulty
        .as_ref()
        .map(|difficulty| difficulty.mode.key())
        .unwrap_or("unknown");
    let difficulty = event
        .difficulty
        .as_ref()
        .map(|difficulty| difficulty.difficulty.key())
        .unwrap_or("unknown");

    format!(
        "reward_event call={call} rank={} berry={berry} mission={mission} mode={mode} difficulty={difficulty}",
        event.rank
    )
}

pub(super) fn reward_mutations_log(mutations: &[RuntimeMutation]) -> String {
    let entries = mutations
        .iter()
        .map(format_runtime_mutation)
        .collect::<Vec<_>>()
        .join(", ");

    format!("reward_mutations count={} [{}]", mutations.len(), entries)
}

pub(super) fn reward_event_payload(
    event: &RewardCommitEvent,
    mutations: &[RuntimeMutation],
) -> RewardEventPayload {
    RewardEventPayload {
        schema: "sdk.runtime.rewards.event.v1",
        rank: event.rank.to_string(),
        berry: event.rewards.berry.map(|reward| reward.amount),
        mission_id: event.mission_id,
        mode: event
            .difficulty
            .as_ref()
            .map(|difficulty| difficulty.mode.key().to_string()),
        difficulty: event
            .difficulty
            .as_ref()
            .map(|difficulty| difficulty.difficulty.key().to_string()),
        mutations: mutations.iter().map(reward_mutation_payload).collect(),
    }
}

fn format_runtime_mutation(mutation: &RuntimeMutation) -> String {
    match mutation {
        RuntimeMutation::Reward(mutation) => format_reward_mutation(mutation),
        RuntimeMutation::Rank(_) => "rank_mutation".to_string(),
        RuntimeMutation::Difficulty(_) => "difficulty_mutation".to_string(),
        RuntimeMutation::Player(_) => "player_mutation".to_string(),
    }
}

fn format_reward_mutation(mutation: &RewardMutation) -> String {
    match mutation {
        RewardMutation::MultiplyBerry(factor) => {
            format!("multiply_berry={}", compact_float(*factor))
        }
        RewardMutation::AddBerry(amount) => format!("add_berry={amount}"),
        RewardMutation::SetBerry(amount) => format!("set_berry={amount}"),
        RewardMutation::AddCrewPoints(amount) => format!("add_crew_points={amount}"),
        RewardMutation::ForceAddSouls(souls) => format!("force_add_souls={}", souls.len()),
        RewardMutation::ForceAddMedals(medals) => format!("force_add_medals={}", medals.len()),
    }
}

fn compact_float(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn reward_mutation_payload(mutation: &RuntimeMutation) -> RewardMutationPayload {
    match mutation {
        RuntimeMutation::Reward(mutation) => match mutation {
            RewardMutation::MultiplyBerry(factor) => RewardMutationPayload {
                kind: "multiply_berry",
                factor: Some(*factor),
                amount_u64: None,
                amount_u32: None,
                souls: Vec::new(),
                medals: Vec::new(),
            },
            RewardMutation::AddBerry(amount) => RewardMutationPayload {
                kind: "add_berry",
                factor: None,
                amount_u64: Some(*amount),
                amount_u32: None,
                souls: Vec::new(),
                medals: Vec::new(),
            },
            RewardMutation::SetBerry(amount) => RewardMutationPayload {
                kind: "set_berry",
                factor: None,
                amount_u64: Some(*amount),
                amount_u32: None,
                souls: Vec::new(),
                medals: Vec::new(),
            },
            RewardMutation::AddCrewPoints(amount) => RewardMutationPayload {
                kind: "add_crew_points",
                factor: None,
                amount_u64: None,
                amount_u32: Some(*amount),
                souls: Vec::new(),
                medals: Vec::new(),
            },
            RewardMutation::ForceAddSouls(souls) => RewardMutationPayload {
                kind: "force_add_souls",
                factor: None,
                amount_u64: None,
                amount_u32: None,
                souls: souls.iter().map(SoulRewardPayload::from).collect(),
                medals: Vec::new(),
            },
            RewardMutation::ForceAddMedals(medals) => RewardMutationPayload {
                kind: "force_add_medals",
                factor: None,
                amount_u64: None,
                amount_u32: None,
                souls: Vec::new(),
                medals: medals.iter().map(MedalRewardPayload::from).collect(),
            },
        },
        RuntimeMutation::Rank(_) => RewardMutationPayload::observation_only("rank_mutation"),
        RuntimeMutation::Difficulty(_) => {
            RewardMutationPayload::observation_only("difficulty_mutation")
        }
        RuntimeMutation::Player(_) => RewardMutationPayload::observation_only("player_mutation"),
    }
}

#[derive(Debug, Serialize)]
pub(super) struct RewardEventPayload {
    schema: &'static str,
    rank: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    berry: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mission_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    difficulty: Option<String>,
    mutations: Vec<RewardMutationPayload>,
}

#[derive(Debug, Serialize)]
struct RewardMutationPayload {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    factor: Option<f64>,
    #[serde(rename = "amount", skip_serializing_if = "Option::is_none")]
    amount_u64: Option<u64>,
    #[serde(rename = "amount", skip_serializing_if = "Option::is_none")]
    amount_u32: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    souls: Vec<SoulRewardPayload>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    medals: Vec<MedalRewardPayload>,
}

impl RewardMutationPayload {
    fn observation_only(kind: &'static str) -> Self {
        Self {
            kind,
            factor: None,
            amount_u64: None,
            amount_u32: None,
            souls: Vec::new(),
            medals: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SoulRewardPayload {
    soul_id: String,
    count: u32,
}

impl From<&SoulReward> for SoulRewardPayload {
    fn from(reward: &SoulReward) -> Self {
        Self {
            soul_id: reward.soul_id.clone(),
            count: reward.count,
        }
    }
}

#[derive(Debug, Serialize)]
struct MedalRewardPayload {
    medal_id: String,
    count: u32,
}

impl From<&MedalReward> for MedalRewardPayload {
    fn from(reward: &MedalReward) -> Self {
        Self {
            medal_id: reward.medal_id.clone(),
            count: reward.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::core::{
        difficulty::{DifficultyId, DifficultyMode, DifficultySnapshot},
        rank::RankValue,
        rewards::RewardState,
    };

    #[test]
    fn reward_event_log_is_compact_and_stable() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(321))
            .with_mission_id(35)
            .with_difficulty(
                DifficultySnapshot::new(
                    DifficultyMode::new("treasure-log"),
                    DifficultyId::new("super-hard"),
                )
                .with_mission_id(35),
            );

        assert_eq!(
            reward_event_log(7, &event),
            "reward_event call=7 rank=S+ berry=321 mission=35 mode=treasure_log difficulty=super_hard"
        );
    }

    #[test]
    fn reward_mutations_log_is_compact_and_stable() {
        let mutations = [
            RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0)),
            RuntimeMutation::Reward(RewardMutation::AddBerry(50)),
        ];

        assert_eq!(
            reward_mutations_log(&mutations),
            "reward_mutations count=2 [multiply_berry=2, add_berry=50]"
        );
    }

    #[test]
    fn reward_event_payload_is_serializable_and_keeps_mutations() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(321));
        let mutations = [RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0))];

        let json = serde_json::to_string(&reward_event_payload(&event, &mutations)).expect("json");

        assert!(json.contains(r#""rank":"S+""#));
        assert!(json.contains(r#""berry":321"#));
        assert!(json.contains(r#""mutations":[{"kind":"multiply_berry","factor":2.0}]"#));
    }
}
