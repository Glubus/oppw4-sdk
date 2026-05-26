use plugin_abi::Oppw4PluginApi;
use serde::{Deserialize, Serialize};

use crate::{PluginError, PluginResult};

pub const DIFFICULTY_SET_RULE: &str = "sdk.runtime.difficulty.set_rule";

#[derive(Clone, Copy)]
pub struct DifficultyService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> DifficultyService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn set_rule(self, rule: DifficultyRule) -> PluginResult<()> {
        let bytes = serde_json::to_vec(&rule)
            .map_err(|error| PluginError::InitFailed(error.to_string()))?;
        super::SignalService::new(self.abi).emit_bytes(DIFFICULTY_SET_RULE, &bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DifficultyRule {
    pub levels: Vec<DifficultyLevel>,
    pub condition: DifficultyConditionExpr,
    pub action: DifficultyAction,
    pub enabled: bool,
}

impl DifficultyRule {
    pub fn new(action: DifficultyAction) -> Self {
        Self {
            levels: Vec::new(),
            condition: DifficultyConditionExpr::None,
            action,
            enabled: true,
        }
    }

    pub fn level(mut self, level: impl Into<DifficultyLevel>) -> Self {
        self.levels.push(level.into());
        self
    }

    pub fn levels<I, L>(mut self, levels: I) -> Self
    where
        I: IntoIterator<Item = L>,
        L: Into<DifficultyLevel>,
    {
        self.levels.extend(levels.into_iter().map(Into::into));
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn condition(mut self, condition: impl Into<DifficultyConditionExpr>) -> Self {
        self.condition = condition.into();
        self
    }

    pub fn when(self, condition: DifficultyCondition) -> Self {
        self.condition(DifficultyConditionExpr::all([condition]))
    }

    pub fn all<I>(self, conditions: I) -> Self
    where
        I: IntoIterator<Item = DifficultyCondition>,
    {
        self.condition(DifficultyConditionExpr::all(conditions))
    }

    pub fn any<I>(self, conditions: I) -> Self
    where
        I: IntoIterator<Item = DifficultyCondition>,
    {
        self.condition(DifficultyConditionExpr::any(conditions))
    }

    pub fn enable_levels<I, L>(levels: I) -> Self
    where
        I: IntoIterator<Item = L>,
        L: Into<DifficultyLevel>,
    {
        Self::new(DifficultyAction::EnableLevel).levels(levels)
    }

    pub fn disable_levels<I, L>(levels: I) -> Self
    where
        I: IntoIterator<Item = L>,
        L: Into<DifficultyLevel>,
    {
        Self::new(DifficultyAction::DisableLevel).levels(levels)
    }

    pub fn scale_actor_stat(stat: impl Into<DifficultyActorStat>, factor: f32) -> Self {
        Self::new(DifficultyAction::ActorStat {
            stat: stat.into(),
            operation: DifficultyValueOp::ScaleF32(factor),
        })
    }

    pub fn set_actor_stat(stat: impl Into<DifficultyActorStat>, value: f32) -> Self {
        Self::new(DifficultyAction::ActorStat {
            stat: stat.into(),
            operation: DifficultyValueOp::SetF32(value),
        })
    }

    pub fn scale_combat_pressure(factor: f32) -> Self {
        Self::new(DifficultyAction::CombatPressure {
            operation: DifficultyValueOp::ScaleF32(factor),
        })
    }

    pub fn set_combat_pressure(value: f32) -> Self {
        Self::new(DifficultyAction::CombatPressure {
            operation: DifficultyValueOp::SetF32(value),
        })
    }

    pub fn known_table(
        table: impl Into<DifficultyKnownTable>,
        operation: DifficultyValueOp,
    ) -> Self {
        Self::new(DifficultyAction::KnownTable {
            table: table.into(),
            operation,
        })
    }

    pub fn raw_fixed_table(
        area: impl Into<DifficultyFixedArea>,
        offset: usize,
        operation: DifficultyValueOp,
    ) -> Self {
        Self::new(DifficultyAction::RawFixedTable {
            area: area.into(),
            offset,
            operation,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DifficultyAction {
    EnableLevel,
    DisableLevel,
    ActorStat {
        stat: DifficultyActorStat,
        operation: DifficultyValueOp,
    },
    CombatPressure {
        operation: DifficultyValueOp,
    },
    KnownTable {
        table: DifficultyKnownTable,
        operation: DifficultyValueOp,
    },
    RawFixedTable {
        area: DifficultyFixedArea,
        offset: usize,
        operation: DifficultyValueOp,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "value", rename_all = "snake_case")]
pub enum DifficultyValueOp {
    SetF32(f32),
    AddF32(f32),
    ScaleF32(f32),
    SetU16(u16),
    AddI16(i16),
    SetU8(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "conditions", rename_all = "snake_case")]
pub enum DifficultyConditionExpr {
    None,
    All(Vec<DifficultyCondition>),
    Any(Vec<DifficultyCondition>),
}

impl DifficultyConditionExpr {
    pub const fn none() -> Self {
        Self::None
    }

    pub fn all<I>(conditions: I) -> Self
    where
        I: IntoIterator<Item = DifficultyCondition>,
    {
        Self::All(conditions.into_iter().collect())
    }

    pub fn any<I>(conditions: I) -> Self
    where
        I: IntoIterator<Item = DifficultyCondition>,
    {
        Self::Any(conditions.into_iter().collect())
    }
}

impl From<Option<DifficultyCondition>> for DifficultyConditionExpr {
    fn from(condition: Option<DifficultyCondition>) -> Self {
        match condition {
            Some(condition) => Self::All(vec![condition]),
            None => Self::None,
        }
    }
}

impl From<DifficultyCondition> for DifficultyConditionExpr {
    fn from(condition: DifficultyCondition) -> Self {
        Self::All(vec![condition])
    }
}

impl From<Vec<DifficultyCondition>> for DifficultyConditionExpr {
    fn from(conditions: Vec<DifficultyCondition>) -> Self {
        Self::All(conditions)
    }
}

impl From<()> for DifficultyConditionExpr {
    fn from((): ()) -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DifficultyCondition {
    Always,
    Flag { key: String, value: bool },
    Equals { key: String, value: String },
    ActiveCharacter { id: String },
    Custom { key: String, value: String },
}

impl DifficultyCondition {
    pub fn flag(key: impl Into<String>, value: bool) -> Self {
        Self::Flag {
            key: key.into(),
            value,
        }
    }

    pub fn equals(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Equals {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn active_character(id: impl Into<String>) -> Self {
        Self::ActiveCharacter { id: id.into() }
    }

    pub fn custom(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Custom {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DifficultyLevel(String);

impl DifficultyLevel {
    pub fn new(level: impl Into<String>) -> Self {
        Self(normalize_difficulty_id(level.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DifficultyLevel {
    fn from(level: &str) -> Self {
        Self::new(level)
    }
}

impl From<String> for DifficultyLevel {
    fn from(level: String) -> Self {
        Self::new(level)
    }
}

impl From<u8> for DifficultyLevel {
    fn from(level: u8) -> Self {
        Self::new(level.to_string())
    }
}

impl From<u32> for DifficultyLevel {
    fn from(level: u32) -> Self {
        Self::new(level.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DifficultyActorStat(String);

impl DifficultyActorStat {
    pub fn new(stat: impl Into<String>) -> Self {
        Self(normalize_actor_stat(stat.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DifficultyActorStat {
    fn from(stat: &str) -> Self {
        Self::new(stat)
    }
}

impl From<String> for DifficultyActorStat {
    fn from(stat: String) -> Self {
        Self::new(stat)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DifficultyKnownTable(String);

impl DifficultyKnownTable {
    pub fn new(table: impl Into<String>) -> Self {
        Self(normalize_known_table(table.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DifficultyKnownTable {
    fn from(table: &str) -> Self {
        Self::new(table)
    }
}

impl From<String> for DifficultyKnownTable {
    fn from(table: String) -> Self {
        Self::new(table)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DifficultyFixedArea(String);

impl DifficultyFixedArea {
    pub fn new(area: impl Into<String>) -> Self {
        Self(normalize_fixed_area(area.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DifficultyFixedArea {
    fn from(area: &str) -> Self {
        Self::new(area)
    }
}

impl From<String> for DifficultyFixedArea {
    fn from(area: String) -> Self {
        Self::new(area)
    }
}

fn normalize_difficulty_id(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "0" | "easy" => "easy".to_string(),
        "1" | "normal" => "normal".to_string(),
        "2" | "hard" => "hard".to_string(),
        "3" | "super_hard" | "super hard" | "super-hard" => "super_hard".to_string(),
        other => other.to_string(),
    }
}

fn normalize_actor_stat(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "health" | "life" | "hp" => "hp".to_string(),
        "atk" | "attack" | "damage" => "attack".to_string(),
        "def" | "defense" | "defence" => "defense".to_string(),
        "stun" | "stagger" | "guard_break" | "guard-break" => "stagger".to_string(),
        other => other.to_string(),
    }
}

fn normalize_known_table(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "combat" | "pressure" | "combat_pressure" | "combat-pressure" | "behavior_scalar" => {
            "combat_pressure".to_string()
        }
        "behavior_chance_a" | "chance_a" | "0x8c" => "behavior_chance_a".to_string(),
        "behavior_chance_b" | "chance_b" | "0x9c" => "behavior_chance_b".to_string(),
        "spawn_b3_a" | "b3_a" | "0xb3d8" => "spawn_b3_a".to_string(),
        "spawn_b3_b" | "b3_b" | "0xb3dc" => "spawn_b3_b".to_string(),
        "spawn_b3_c" | "b3_c" | "0xb3e0" => "spawn_b3_c".to_string(),
        "candidate_a" | "0x6608" => "candidate_a".to_string(),
        "candidate_b" | "0x1b08" => "candidate_b".to_string(),
        other => other.to_string(),
    }
}

fn normalize_fixed_area(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "20" | "fixed20" | "fixed_20" | "fixed+0x20" => "fixed20".to_string(),
        "28" | "fixed28" | "fixed_28" | "fixed+0x28" => "fixed28".to_string(),
        "d8" | "fixedd8" | "fixed_d8" | "fixed+0xd8" => "fixedd8".to_string(),
        other => other.to_string(),
    }
}
