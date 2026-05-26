use plugin_abi::Oppw4PluginApi;
use serde::{Deserialize, Serialize};

use crate::{PluginError, PluginResult};

pub const RANK_SET_CAP: &str = "sdk.runtime.rank.set_cap";
pub const RANK_SHIFT_COUNT_THRESHOLDS: &str = "sdk.runtime.rank.shift_count_thresholds";
pub const RANK_OVERRIDE_COUNT_THRESHOLDS: &str = "sdk.runtime.rank.override_count_thresholds";

#[derive(Clone, Copy)]
pub struct RankService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> RankService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn set_rank_cap(self, rule: RankCapRule) -> PluginResult<()> {
        self.emit_json(RANK_SET_CAP, &rule)
    }

    pub fn shift_count_thresholds(self, request: CountThresholdShift) -> PluginResult<()> {
        self.emit_json(RANK_SHIFT_COUNT_THRESHOLDS, &request)
    }

    pub fn override_count_thresholds(self, request: CountThresholdOverride) -> PluginResult<()> {
        self.emit_json(RANK_OVERRIDE_COUNT_THRESHOLDS, &request)
    }

    fn emit_json<T: Serialize>(self, signal: &str, payload: &T) -> PluginResult<()> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|error| PluginError::InitFailed(error.to_string()))?;
        super::SignalService::new(self.abi).emit_bytes(signal, &bytes)
    }
}

/// Rule sent to `sdk.runtime` to change rank cap behavior.
///
/// The rule targets reward rank slots directly. Slots are the real rank values
/// observed in the reward flow: `0..5` maps to `D..S+`. Human aliases such as
/// `"d"`, `"s"`, and `"s_plus"` are normalized to the same internal ids.
///
/// `condition` is explicit: `None` means unconditional, `All` and `Any` carry
/// one or more predicates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankCapRule {
    pub slots: Vec<RankSlot>,
    pub condition: RankConditionExpr,
    pub effect: RankCapEffect,
    pub enabled: bool,
}

impl RankCapRule {
    pub fn new(effect: RankCapEffect) -> Self {
        Self {
            slots: Vec::new(),
            condition: RankConditionExpr::None,
            effect,
            enabled: true,
        }
    }

    pub fn slot(mut self, slot: impl Into<RankSlot>) -> Self {
        self.slots.push(slot.into());
        self
    }

    pub fn slots<I, S>(mut self, slots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<RankSlot>,
    {
        self.slots.extend(slots.into_iter().map(Into::into));
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn condition(mut self, condition: impl Into<RankConditionExpr>) -> Self {
        self.condition = condition.into();
        self
    }

    pub fn when(self, condition: RankCondition) -> Self {
        self.condition(RankConditionExpr::all([condition]))
    }

    pub fn all<I>(self, conditions: I) -> Self
    where
        I: IntoIterator<Item = RankCondition>,
    {
        self.condition(RankConditionExpr::all(conditions))
    }

    pub fn any<I>(self, conditions: I) -> Self
    where
        I: IntoIterator<Item = RankCondition>,
    {
        self.condition(RankConditionExpr::any(conditions))
    }

    pub fn enable() -> Self {
        Self::new(RankCapEffect::Enable)
    }

    pub fn disable() -> Self {
        Self::new(RankCapEffect::Disable)
    }

    pub fn keep_default() -> Self {
        Self::new(RankCapEffect::KeepDefault)
    }

    pub fn enable_s() -> Self {
        Self::enable().slot("s")
    }

    pub fn enable_s_plus() -> Self {
        Self::enable().slot("s_plus")
    }

    pub fn enable_slots<I, S>(slots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<RankSlot>,
    {
        Self::enable().slots(slots)
    }

    pub fn disable_slots<I, S>(slots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<RankSlot>,
    {
        Self::disable().slots(slots)
    }
}

/// Condition expression attached to a rank rule.
///
/// `None` is the explicit representation of Lua `nil`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "conditions", rename_all = "snake_case")]
pub enum RankConditionExpr {
    None,
    All(Vec<RankCondition>),
    Any(Vec<RankCondition>),
}

impl RankConditionExpr {
    pub const fn none() -> Self {
        Self::None
    }

    pub fn all<I>(conditions: I) -> Self
    where
        I: IntoIterator<Item = RankCondition>,
    {
        Self::All(conditions.into_iter().collect())
    }

    pub fn any<I>(conditions: I) -> Self
    where
        I: IntoIterator<Item = RankCondition>,
    {
        Self::Any(conditions.into_iter().collect())
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl From<Option<RankCondition>> for RankConditionExpr {
    fn from(condition: Option<RankCondition>) -> Self {
        match condition {
            Some(condition) => Self::All(vec![condition]),
            None => Self::None,
        }
    }
}

impl From<RankCondition> for RankConditionExpr {
    fn from(condition: RankCondition) -> Self {
        Self::All(vec![condition])
    }
}

impl From<Vec<RankCondition>> for RankConditionExpr {
    fn from(conditions: Vec<RankCondition>) -> Self {
        Self::All(conditions)
    }
}

impl From<()> for RankConditionExpr {
    fn from((): ()) -> Self {
        Self::None
    }
}

/// Reward rank slot.
///
/// Known slots are:
///
/// - `0` / `"d"`: D
/// - `1` / `"c"`: C
/// - `2` / `"b"`: B
/// - `3` / `"a"`: A
/// - `4` / `"s"`: S
/// - `5` / `"s_plus"` / `"s+"`: S+
///
/// Unknown strings are preserved so future runtime hooks can introduce custom
/// slots without changing the SDK ABI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RankSlot(String);

impl RankSlot {
    pub fn new(slot: impl Into<String>) -> Self {
        Self(normalize_rank_id(slot.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn d() -> Self {
        Self::new("d")
    }

    pub fn c() -> Self {
        Self::new("c")
    }

    pub fn b() -> Self {
        Self::new("b")
    }

    pub fn a() -> Self {
        Self::new("a")
    }

    pub fn s() -> Self {
        Self::new("s")
    }

    pub fn s_plus() -> Self {
        Self::new("s_plus")
    }
}

impl From<&str> for RankSlot {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RankSlot {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<u32> for RankSlot {
    fn from(value: u32) -> Self {
        Self::new(value.to_string())
    }
}

impl From<usize> for RankSlot {
    fn from(value: usize) -> Self {
        Self::new(value.to_string())
    }
}

fn normalize_rank_id(value: String) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "0" | "d" => "d".to_string(),
        "1" | "c" => "c".to_string(),
        "2" | "b" => "b".to_string(),
        "3" | "a" => "a".to_string(),
        "4" | "s" => "s".to_string(),
        "5" | "s+" | "s_plus" | "splus" => "s_plus".to_string(),
        _ => value,
    }
}

/// Predicate used inside a rank condition expression.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RankCondition {
    Always,
    Flag { key: String, value: bool },
    Equals { key: String, value: String },
    ActiveCharacter { id: String },
    Custom { key: String, value: String },
}

impl RankCondition {
    pub const fn always() -> Self {
        Self::Always
    }

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

/// Effect applied to the selected rank slots when conditions match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankCapEffect {
    KeepDefault,
    Enable,
    Disable,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountThresholdShift {
    pub rank_row_ids: Vec<u16>,
    pub source_prefix: [u32; 3],
    pub inserted_first: u32,
    pub inserted_second: Option<u32>,
}

impl CountThresholdShift {
    pub const DEFAULT_SOURCE_PREFIX: [u32; 3] = [60_000, 60_000, 48_000];
    pub const DEFAULT_INSERTED_FIRST: u32 = 72_000;

    pub fn new(rank_row_ids: Vec<u16>) -> Self {
        Self {
            rank_row_ids,
            source_prefix: Self::DEFAULT_SOURCE_PREFIX,
            inserted_first: Self::DEFAULT_INSERTED_FIRST,
            inserted_second: None,
        }
    }

    pub const fn source_prefix(mut self, source_prefix: [u32; 3]) -> Self {
        self.source_prefix = source_prefix;
        self
    }

    pub const fn inserted_first(mut self, inserted_first: u32) -> Self {
        self.inserted_first = inserted_first;
        self
    }

    pub const fn inserted_second(mut self, inserted_second: u32) -> Self {
        self.inserted_second = Some(inserted_second);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountThresholdOverride {
    pub rank_row_ids: Vec<u16>,
    pub source_prefix: [u32; 3],
    pub thresholds: [u32; 5],
}

impl CountThresholdOverride {
    pub fn new(rank_row_ids: Vec<u16>, thresholds: [u32; 5]) -> Self {
        Self {
            rank_row_ids,
            source_prefix: CountThresholdShift::DEFAULT_SOURCE_PREFIX,
            thresholds,
        }
    }

    pub const fn source_prefix(mut self, source_prefix: [u32; 3]) -> Self {
        self.source_prefix = source_prefix;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{c_char, c_void};

    use plugin_abi::null_api;

    use super::*;

    unsafe extern "system" fn emit_signal(
        _host_context: *mut c_void,
        _signal_utf8: *const c_char,
        _payload: *const u8,
        payload_len: usize,
    ) -> i32 {
        if payload_len > 0 {
            0
        } else {
            -1
        }
    }

    #[test]
    fn rank_service_emits_json_command() {
        let mut api = null_api();
        api.emit_signal = Some(emit_signal);

        let result = RankService::new(&api).set_rank_cap(RankCapRule::enable().slots([4_u32, 5]));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn rank_slots_accept_numbers_and_letters() {
        assert_eq!(RankSlot::from(0_u32), RankSlot::d());
        assert_eq!(RankSlot::from("D"), RankSlot::d());
        assert_eq!(RankSlot::from(4_u32), RankSlot::s());
        assert_eq!(RankSlot::from("S+"), RankSlot::s_plus());
    }

    #[test]
    fn rank_rule_supports_nil_all_and_any_conditions() {
        let none = RankCapRule::enable()
            .slot("a")
            .condition(Option::<RankCondition>::None);
        assert_eq!(none.condition, RankConditionExpr::None);

        let all = RankCapRule::enable().slot("s").all([
            RankCondition::active_character("zoro"),
            RankCondition::flag("crew.elbaph", true),
        ]);
        assert!(matches!(all.condition, RankConditionExpr::All(_)));

        let any = RankCapRule::disable()
            .slot("s_plus")
            .any([RankCondition::equals("mission", "elbaph")]);
        assert!(matches!(any.condition, RankConditionExpr::Any(_)));
    }
}
