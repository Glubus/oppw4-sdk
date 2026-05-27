use std::fmt;

use super::{difficulty::DifficultySnapshot, player::PlayerSnapshot};

/// Result rank value as exposed to mods.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum RankValue {
    D,
    C,
    B,
    A,
    S,
    SPlus,
    Unknown(u8),
}

impl RankValue {
    /// Converts a raw game slot into a modder-facing rank.
    pub(crate) const fn from_slot(slot: u8) -> Self {
        match slot {
            0 => Self::D,
            1 => Self::C,
            2 => Self::B,
            3 => Self::A,
            4 => Self::S,
            5 => Self::SPlus,
            value => Self::Unknown(value),
        }
    }

    /// Human-readable debug alias.
    pub(crate) const fn debug_alias(self) -> &'static str {
        match self {
            Self::D => "D",
            Self::C => "C",
            Self::B => "B",
            Self::A => "A",
            Self::S => "S",
            Self::SPlus => "S+",
            Self::Unknown(_) => "unknown",
        }
    }
}

impl fmt::Display for RankValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(slot) => write!(formatter, "unknown({slot})"),
            rank => formatter.write_str(rank.debug_alias()),
        }
    }
}

/// Result-screen rank context emitted by the core runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RankResultEvent {
    pub(crate) rank: RankValue,
    pub(crate) mission_id: Option<u32>,
    pub(crate) difficulty: Option<DifficultySnapshot>,
    pub(crate) player: PlayerSnapshot,
}

impl RankResultEvent {
    pub(crate) const fn new(rank: RankValue) -> Self {
        Self {
            rank,
            mission_id: None,
            difficulty: None,
            player: PlayerSnapshot::new(),
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
        self.player = player;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_five_is_s_plus() {
        assert_eq!(RankValue::from_slot(5), RankValue::SPlus);
        assert_eq!(RankValue::SPlus.debug_alias(), "S+");
    }

    #[test]
    fn unknown_slot_is_preserved() {
        assert_eq!(RankValue::from_slot(9), RankValue::Unknown(9));
    }

    #[test]
    fn rank_result_event_carries_player_snapshot() {
        let player = PlayerSnapshot::new().with_active_character("zoro");
        let event = RankResultEvent::new(RankValue::SPlus)
            .with_mission_id(35)
            .with_player(player.clone());

        assert_eq!(event.rank, RankValue::SPlus);
        assert_eq!(event.mission_id, Some(35));
        assert_eq!(event.player, player);
    }
}
