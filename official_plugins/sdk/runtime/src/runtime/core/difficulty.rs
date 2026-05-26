/// Current game mode associated with a difficulty snapshot.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DifficultyMode {
    Story,
    TreasureLog,
    FreeLog,
    Unknown(String),
}

impl DifficultyMode {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        match value.into().as_str() {
            "0" => Self::Story,
            "1" => Self::FreeLog,
            "2" => Self::TreasureLog,
            "story" => Self::Story,
            "treasure_log" | "treasure-log" => Self::TreasureLog,
            "free_log" | "free-log" => Self::FreeLog,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub(crate) fn key(&self) -> &str {
        match self {
            Self::Story => "story",
            Self::TreasureLog => "treasure_log",
            Self::FreeLog => "free_log",
            Self::Unknown(value) => value,
        }
    }
}

/// Modder-facing difficulty id.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DifficultyId {
    Easy,
    Normal,
    Hard,
    SuperHard,
    Unknown(String),
}

impl DifficultyId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        match value.into().as_str() {
            "0" => Self::Easy,
            "1" => Self::Normal,
            "2" => Self::Hard,
            "3" => Self::SuperHard,
            "easy" => Self::Easy,
            "normal" => Self::Normal,
            "hard" => Self::Hard,
            "super_hard" | "super-hard" => Self::SuperHard,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub(crate) fn key(&self) -> &str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
            Self::SuperHard => "super_hard",
            Self::Unknown(value) => value,
        }
    }
}

/// Runtime difficulty context shared with event handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DifficultySnapshot {
    pub(crate) mission_id: Option<u32>,
    pub(crate) mode: DifficultyMode,
    pub(crate) difficulty: DifficultyId,
}

/// Difficulty application point emitted before supported difficulty mutations are applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DifficultyApplyEvent {
    pub(crate) snapshot: DifficultySnapshot,
}

impl DifficultyApplyEvent {
    pub(crate) const fn new(snapshot: DifficultySnapshot) -> Self {
        Self { snapshot }
    }
}

/// Numeric operation supported by confirmed difficulty table mutations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DifficultyValueOp {
    SetF32(f32),
    AddF32(f32),
    ScaleF32(f32),
    SetU16(u16),
    AddI16(i16),
    SetU8(u8),
}

/// Mutation requested by difficulty handlers.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DifficultyMutation {
    CombatPressure {
        operation: DifficultyValueOp,
    },
    KnownTable {
        table: String,
        operation: DifficultyValueOp,
    },
    UnsupportedActorStat {
        stat: String,
    },
}

impl DifficultySnapshot {
    pub(crate) const fn new(mode: DifficultyMode, difficulty: DifficultyId) -> Self {
        Self {
            mission_id: None,
            mode,
            difficulty,
        }
    }

    pub(crate) const fn with_mission_id(mut self, mission_id: u32) -> Self {
        self.mission_id = Some(mission_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_snapshot_keeps_public_names() {
        let snapshot = DifficultySnapshot::new(
            DifficultyMode::new("treasure-log"),
            DifficultyId::new("super-hard"),
        )
        .with_mission_id(35);

        assert_eq!(snapshot.mission_id, Some(35));
        assert_eq!(snapshot.mode.key(), "treasure_log");
        assert_eq!(snapshot.difficulty.key(), "super_hard");
    }

    #[test]
    fn difficulty_apply_event_wraps_snapshot() {
        let snapshot = DifficultySnapshot::new(
            DifficultyMode::new("treasure_log"),
            DifficultyId::new("hard"),
        )
        .with_mission_id(35);

        let event = DifficultyApplyEvent::new(snapshot.clone());

        assert_eq!(event.snapshot, snapshot);
    }
}
