use super::{
    difficulty::{DifficultyApplyEvent, DifficultyMutation},
    player::{PlayerChangeEvent, PlayerMutation},
    rank::{RankMutation, RankResultEvent},
    rewards::{RewardCommitEvent, RewardMutation},
};

/// Typed runtime event emitted by gameplay hooks before any script frontend is involved.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RuntimeEvent {
    RewardCommit(RewardCommitEvent),
    RankResult(RankResultEvent),
    DifficultyApply(DifficultyApplyEvent),
    PlayerChange(PlayerChangeEvent),
}

/// Typed mutation requested by a runtime event handler.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RuntimeMutation {
    Reward(RewardMutation),
    Rank(RankMutation),
    Difficulty(DifficultyMutation),
    Player(PlayerMutation),
}

impl From<RewardCommitEvent> for RuntimeEvent {
    fn from(event: RewardCommitEvent) -> Self {
        Self::RewardCommit(event)
    }
}

impl From<RewardMutation> for RuntimeMutation {
    fn from(mutation: RewardMutation) -> Self {
        Self::Reward(mutation)
    }
}

impl From<RankResultEvent> for RuntimeEvent {
    fn from(event: RankResultEvent) -> Self {
        Self::RankResult(event)
    }
}

impl From<RankMutation> for RuntimeMutation {
    fn from(mutation: RankMutation) -> Self {
        Self::Rank(mutation)
    }
}

impl From<DifficultyApplyEvent> for RuntimeEvent {
    fn from(event: DifficultyApplyEvent) -> Self {
        Self::DifficultyApply(event)
    }
}

impl From<DifficultyMutation> for RuntimeMutation {
    fn from(mutation: DifficultyMutation) -> Self {
        Self::Difficulty(mutation)
    }
}

impl From<PlayerChangeEvent> for RuntimeEvent {
    fn from(event: PlayerChangeEvent) -> Self {
        Self::PlayerChange(event)
    }
}

impl From<PlayerMutation> for RuntimeMutation {
    fn from(mutation: PlayerMutation) -> Self {
        Self::Player(mutation)
    }
}
