use super::rank::RankValue;

/// Runtime event emitted when the game commits result rewards.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RewardCommitEvent {
    pub(crate) rank: RankValue,
    pub(crate) rewards: RewardState,
}

impl RewardCommitEvent {
    pub(crate) const fn new(rank: RankValue, rewards: RewardState) -> Self {
        Self { rank, rewards }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MedalReward {
    pub(crate) medal_id: String,
    pub(crate) count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_commit_event_can_omit_mission() {
        let event = RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100));

        assert_eq!(event.rank, RankValue::SPlus);
        assert_eq!(event.rewards.berry, Some(BerryReward { amount: 100 }));
    }
}
