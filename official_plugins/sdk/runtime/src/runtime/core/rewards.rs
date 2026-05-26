use super::{difficulty::DifficultySnapshot, player::PlayerSnapshot, rank::RankValue};

/// Runtime event emitted when the game commits result rewards.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RewardCommitEvent {
    pub(crate) rank: RankValue,
    pub(crate) mission_id: Option<u32>,
    pub(crate) difficulty: Option<DifficultySnapshot>,
    pub(crate) player: Option<PlayerSnapshot>,
    pub(crate) rewards: RewardState,
}

impl RewardCommitEvent {
    pub(crate) const fn new(rank: RankValue, rewards: RewardState) -> Self {
        Self {
            rank,
            mission_id: None,
            difficulty: None,
            player: None,
            rewards,
        }
    }

    pub(crate) const fn with_mission_id(mut self, mission_id: u32) -> Self {
        self.mission_id = Some(mission_id);
        self
    }

    pub(crate) fn with_difficulty(mut self, difficulty: DifficultySnapshot) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    pub(crate) fn with_player(mut self, player: PlayerSnapshot) -> Self {
        self.player = Some(player);
        self
    }
}

/// Typed reward mutation produced by script frontends and applied by the core.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RewardMutation {
    MultiplyBerry(f64),
    AddBerry(u64),
    SetBerry(u64),
    AddCrewPoints(u32),
    ForceAddSouls(Vec<SoulReward>),
    ForceAddMedals(Vec<MedalReward>),
}

/// Result reward state visible to mods.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RewardState {
    pub(crate) berry: Option<BerryReward>,
    pub(crate) souls: Vec<SoulReward>,
    pub(crate) medals: Vec<MedalReward>,
    pub(crate) crew_points: Option<CrewPointReward>,
}

impl RewardState {
    pub(crate) const fn new() -> Self {
        Self {
            berry: None,
            souls: Vec::new(),
            medals: Vec::new(),
            crew_points: None,
        }
    }

    pub(crate) const fn with_berry(mut self, amount: u64) -> Self {
        self.berry = Some(BerryReward { amount });
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BerryReward {
    pub(crate) amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CrewPointReward {
    pub(crate) amount: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SoulReward {
    pub(crate) soul_id: String,
    pub(crate) count: u32,
}

impl SoulReward {
    pub(crate) fn new(soul_id: impl Into<String>, count: u32) -> Self {
        Self {
            soul_id: soul_id.into(),
            count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MedalReward {
    pub(crate) medal_id: String,
    pub(crate) count: u32,
}

impl MedalReward {
    pub(crate) fn new(medal_id: impl Into<String>, count: u32) -> Self {
        Self {
            medal_id: medal_id.into(),
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_berry_mutation_is_vec_storable() {
        let mutations = vec![RewardMutation::MultiplyBerry(2.0)];

        assert_eq!(mutations, [RewardMutation::MultiplyBerry(2.0)]);
    }

    #[test]
    fn reward_commit_event_can_omit_mission() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100));

        assert_eq!(event.rank, RankValue::SPlus);
        assert_eq!(event.mission_id, None);
        assert_eq!(event.rewards.berry, Some(BerryReward { amount: 100 }));
    }
}
