use std::sync::Arc;

use mlua::{Lua, Table, Value};
use plugin_sdk::OwnedHostApi;
use serde::Serialize;
use serde_json::json;
use serde_json::Value as JsonValue;
use struct_api::missions::Mission;

use crate::runtime::signals;

pub(super) const MODULE_NAME: &str = "sdk.runtime.rewards";

#[derive(Clone, Copy)]
enum RewardTarget {
    Berry,
    Souls,
    CrewPoints,
    Medals,
}

impl RewardTarget {
    const fn name(self) -> &'static str {
        match self {
            Self::Berry => "berry",
            Self::Souls => "souls",
            Self::CrewPoints => "crew_points",
            Self::Medals => "medals",
        }
    }
}

#[derive(Serialize)]
struct RewardRule {
    kind: &'static str,
    target: &'static str,
    action: RewardAction,
    condition: JsonValue,
    enabled: bool,
    stub: bool,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RewardAction {
    ForceAdd {
        missing_only: bool,
        minimum: u32,
        rewards: Rewards,
    },
    Multiply {
        factor: f64,
    },
}

#[derive(Default, Serialize)]
struct Rewards {
    #[serde(skip_serializing_if = "Option::is_none")]
    reward_souls: Option<RewardSouls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reward_berry: Option<RewardBerry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reward_medals: Option<RewardMedals>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reward_crew_points: Option<RewardCrewPoints>,
}

#[derive(Serialize)]
struct RewardBerry {
    amount: u32,
}

#[derive(Serialize)]
struct RewardSouls {
    souls: Vec<Soul>,
}

#[derive(Serialize)]
struct Soul {
    #[serde(rename = "type")]
    soul_type: String,
    count: u32,
}

#[derive(Serialize)]
struct RewardCrewPoints {
    amount: u32,
}

#[derive(Serialize)]
struct RewardMedals {
    medals: Vec<Medal>,
}

#[derive(Serialize)]
struct Medal {
    #[serde(rename = "type")]
    medal_type: String,
    count: u32,
}

#[cfg(test)]
pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_context(lua, None)
}

pub(super) fn module_with_host(lua: &Lua, host: Arc<OwnedHostApi>) -> mlua::Result<Table> {
    module_with_context(lua, Some(host))
}

fn module_with_context(lua: &Lua, host: Option<Arc<OwnedHostApi>>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(
        "berry",
        reward_builder(lua, RewardTarget::Berry, Value::Nil, host.clone())?,
    )?;
    table.set(
        "souls",
        reward_builder(lua, RewardTarget::Souls, Value::Nil, host.clone())?,
    )?;
    table.set(
        "crew_points",
        reward_builder(lua, RewardTarget::CrewPoints, Value::Nil, host.clone())?,
    )?;
    table.set(
        "medals",
        reward_builder(lua, RewardTarget::Medals, Value::Nil, host.clone())?,
    )?;
    table.set("rank", rank(lua)?)?;
    table.set("for_mission", lua.create_function(for_mission)?)?;
    table.set("missions", lua.create_function(missions)?)?;
    table.set("all", lua.create_function(all)?)?;
    table.set("any", lua.create_function(any)?)?;
    table.set("flag", lua.create_function(flag)?)?;
    table.set("equals", lua.create_function(equals)?)?;
    table.set("custom", lua.create_function(custom)?)?;
    Ok(table)
}

fn rank(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "RewardRank")?;
    table.set(
        "contains",
        lua.create_function(|lua, (_this, slots): (Table, Value)| rank_contains(lua, slots))?,
    )?;
    Ok(table)
}

fn rank_contains(lua: &Lua, slots: Value) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "rank_contains")?;
    table.set("slots", lua.create_sequence_from(parse_rank_slots(slots)?)?)?;
    Ok(table)
}

fn reward_builder(
    lua: &Lua,
    target: RewardTarget,
    condition: Value,
    host: Option<Arc<OwnedHostApi>>,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", format!("Reward{}", pascal_target_name(target)))?;
    table.set("target", target.name())?;
    table.set("__reward_condition", condition)?;
    table.set("condition", {
        let host = host.clone();
        lua.create_function(move |lua, (_this, condition): (Table, Value)| {
            reward_builder(lua, target, condition, host.clone())
        })?
    })?;
    table.set("force_add", {
        let host = host.clone();
        lua.create_function(move |_, (this, values): (Table, Value)| {
            let condition: Value = this.get("__reward_condition")?;
            let rule = RewardRule {
                kind: "reward_rule",
                target: target.name(),
                action: RewardAction::ForceAdd {
                    missing_only: true,
                    minimum: 1,
                    rewards: parse_rewards(target, values)?,
                },
                condition: parse_condition_expr(condition)?,
                enabled: true,
                stub: true,
            };
            serialize_and_emit_rule(host.as_deref(), &rule)
        })?
    })?;
    table.set("multiply", {
        let host = host.clone();
        lua.create_function(move |_, (this, factor): (Table, Value)| {
            let condition: Value = this.get("__reward_condition")?;
            let rule = RewardRule {
                kind: "reward_rule",
                target: target.name(),
                action: RewardAction::Multiply {
                    factor: parse_factor(factor)?,
                },
                condition: parse_condition_expr(condition)?,
                enabled: true,
                stub: true,
            };
            serialize_and_emit_rule(host.as_deref(), &rule)
        })?
    })?;
    Ok(table)
}

fn serialize_and_emit_rule(host: Option<&OwnedHostApi>, rule: &RewardRule) -> mlua::Result<String> {
    let bytes = serde_json::to_vec(rule).map_err(mlua::Error::external)?;
    if let Some(host) = host {
        host.signals()
            .emit_bytes(signals::REWARD_STAGE_RULE, &bytes)
            .map_err(mlua::Error::external)?;
    }
    String::from_utf8(bytes).map_err(mlua::Error::external)
}

const fn pascal_target_name(target: RewardTarget) -> &'static str {
    match target {
        RewardTarget::Berry => "Berry",
        RewardTarget::Souls => "Souls",
        RewardTarget::CrewPoints => "CrewPoints",
        RewardTarget::Medals => "Medals",
    }
}

fn all(lua: &Lua, values: mlua::MultiValue) -> mlua::Result<Table> {
    condition_expr(lua, "all", values)
}

fn any(lua: &Lua, values: mlua::MultiValue) -> mlua::Result<Table> {
    condition_expr(lua, "any", values)
}

fn flag(lua: &Lua, (key, value): (String, bool)) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "flag")?;
    table.set("key", key)?;
    table.set("value", value)?;
    Ok(table)
}

fn equals(lua: &Lua, (key, value): (String, String)) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "equals")?;
    table.set("key", key)?;
    table.set("value", value)?;
    Ok(table)
}

fn custom(lua: &Lua, (key, value): (String, String)) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "custom")?;
    table.set("key", key)?;
    table.set("value", value)?;
    Ok(table)
}

fn condition_expr(lua: &Lua, mode: &'static str, values: mlua::MultiValue) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("mode", mode)?;
    let conditions = lua.create_table()?;
    for (index, value) in values.into_iter().enumerate() {
        conditions.set(index + 1, value)?;
    }
    table.set("conditions", conditions)?;
    Ok(table)
}

fn for_mission(lua: &Lua, query: Value) -> mlua::Result<Value> {
    let Some(mission) = find_mission(query)? else {
        return Ok(Value::Nil);
    };
    let Some(rewards) = &mission.rewards else {
        return Ok(Value::Nil);
    };

    let table = lua.create_table()?;
    table.set("mission", mission_summary_table(lua, mission)?)?;
    table.set(
        "observations",
        values_to_lua_array(lua, &rewards.observations)?,
    )?;
    Ok(Value::Table(table))
}

fn parse_rewards(target: RewardTarget, values: Value) -> mlua::Result<Rewards> {
    let mut rewards = Rewards::default();
    match target {
        RewardTarget::Berry => {
            rewards.reward_berry = Some(RewardBerry {
                amount: parse_amount(values, "berry amount")?,
            });
        }
        RewardTarget::Souls => {
            rewards.reward_souls = Some(RewardSouls {
                souls: parse_souls(values)?,
            });
        }
        RewardTarget::CrewPoints => {
            rewards.reward_crew_points = Some(RewardCrewPoints {
                amount: parse_amount(values, "crew point amount")?,
            });
        }
        RewardTarget::Medals => {
            rewards.reward_medals = Some(RewardMedals {
                medals: parse_medals(values)?,
            });
        }
    }
    Ok(rewards)
}

fn parse_amount(value: Value, type_name: &'static str) -> mlua::Result<u32> {
    match value {
        Value::Table(table) => parse_minimum_count(table.get::<Value>("amount")?),
        value => parse_u32(value, type_name).map(|count| count.max(1)),
    }
}

fn parse_souls(values: Value) -> mlua::Result<Vec<Soul>> {
    let table = reward_table(values, "souls table")?;
    let mut souls = Vec::new();
    for pair in table.pairs::<String, Value>() {
        let (soul_type, value) = pair?;
        souls.push(Soul {
            soul_type,
            count: parse_minimum_count(value)?,
        });
    }
    souls.sort_by(|left, right| left.soul_type.cmp(&right.soul_type));
    Ok(souls)
}

fn parse_medals(values: Value) -> mlua::Result<Vec<Medal>> {
    let table = reward_table(values, "medals table")?;
    let mut medals = Vec::new();
    for pair in table.pairs::<String, Value>() {
        let (medal_type, value) = pair?;
        medals.push(Medal {
            medal_type,
            count: parse_minimum_count(value)?,
        });
    }
    medals.sort_by(|left, right| left.medal_type.cmp(&right.medal_type));
    Ok(medals)
}

fn reward_table(values: Value, type_name: &'static str) -> mlua::Result<Table> {
    match values {
        Value::Table(table) => Ok(table),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: type_name.to_string(),
            message: None,
        }),
    }
}

fn parse_minimum_count(value: Value) -> mlua::Result<u32> {
    Ok(parse_u32(value, "soul count")?.max(1))
}

fn parse_factor(value: Value) -> mlua::Result<f64> {
    let factor = match value {
        Value::Integer(value) => value as f64,
        Value::Number(value) => value,
        other => {
            return Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "reward multiplier".to_string(),
                message: None,
            });
        }
    };
    Ok(factor.max(1.0))
}

fn parse_u32(value: Value, type_name: &'static str) -> mlua::Result<u32> {
    let value = match value {
        Value::Integer(value) => value,
        other => {
            return Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: type_name.to_string(),
                message: None,
            });
        }
    };
    u32::try_from(value).map_err(mlua::Error::external)
}

fn parse_condition_expr(value: Value) -> mlua::Result<JsonValue> {
    match value {
        Value::Nil => Ok(json!({ "mode": "none" })),
        Value::Table(table) => match table.get::<Option<String>>("mode")?.as_deref() {
            Some("all") => Ok(json!({
                "mode": "all",
                "conditions": parse_condition_list(table)?,
            })),
            Some("any") => Ok(json!({
                "mode": "any",
                "conditions": parse_condition_list(table)?,
            })),
            _ => Ok(json!({
                "mode": "all",
                "conditions": [parse_condition(table)?],
            })),
        },
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "reward condition".to_string(),
            message: None,
        }),
    }
}

fn parse_condition_list(table: Table) -> mlua::Result<Vec<JsonValue>> {
    let conditions: Table = table.get("conditions")?;
    conditions
        .sequence_values::<Table>()
        .map(|condition| parse_condition(condition?))
        .collect()
}

fn parse_condition(table: Table) -> mlua::Result<JsonValue> {
    match table.get::<String>("kind")?.as_str() {
        "rank_contains" => Ok(json!({
            "kind": "rank_contains",
            "slots": lua_string_sequence(table.get::<Table>("slots")?)?,
        })),
        "flag" => Ok(json!({
            "kind": "flag",
            "key": table.get::<String>("key")?,
            "value": table.get::<bool>("value")?,
        })),
        "equals" => Ok(json!({
            "kind": "equals",
            "key": table.get::<String>("key")?,
            "value": table.get::<String>("value")?,
        })),
        "custom" => Ok(json!({
            "kind": "custom",
            "key": table.get::<String>("key")?,
            "value": table.get::<String>("value")?,
        })),
        kind => Err(mlua::Error::external(format!(
            "unknown reward condition kind: {kind}"
        ))),
    }
}

fn parse_rank_slots(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::Integer(slot) => Ok(vec![normalize_rank_slot(slot.to_string())]),
        Value::String(slot) => Ok(vec![normalize_rank_slot(slot.to_str()?.as_ref())]),
        Value::Table(table) => table
            .sequence_values::<Value>()
            .map(parse_rank_slot)
            .collect(),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot or rank slot list".to_string(),
            message: None,
        }),
    }
}

fn parse_rank_slot(value: mlua::Result<Value>) -> mlua::Result<String> {
    match value? {
        Value::Integer(slot) => Ok(normalize_rank_slot(slot.to_string())),
        Value::String(slot) => Ok(normalize_rank_slot(slot.to_str()?.as_ref())),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot".to_string(),
            message: None,
        }),
    }
}

fn normalize_rank_slot(value: impl AsRef<str>) -> String {
    match value
        .as_ref()
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .as_str()
    {
        "0" | "d" => "d".to_string(),
        "1" | "c" => "c".to_string(),
        "2" | "b" => "b".to_string(),
        "3" | "a" => "a".to_string(),
        "4" | "s" => "s".to_string(),
        "5" | "s+" | "s_plus" => "s_plus".to_string(),
        slot => slot.to_string(),
    }
}

fn lua_string_sequence(table: Table) -> mlua::Result<Vec<String>> {
    table.sequence_values::<String>().collect()
}

fn missions(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, mission) in struct_api::missions::all()
        .iter()
        .filter(|mission| mission.rewards.is_some())
        .enumerate()
    {
        table.set(index + 1, mission_summary_table(lua, mission)?)?;
    }
    Ok(table)
}

fn find_mission(query: Value) -> mlua::Result<Option<&'static Mission>> {
    match query {
        Value::String(id) => Ok(struct_api::missions::find(id.to_str()?.as_ref())),
        Value::Integer(id) if (0..=u16::MAX as i64).contains(&id) => {
            Ok(struct_api::missions::find_by_id(id as u16))
        }
        _ => Ok(None),
    }
}

fn mission_summary_table(lua: &Lua, mission: &Mission) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "mission")?;
    table.set("id", mission.id.as_str())?;
    if let Some(display_name) = &mission.display_name {
        table.set("display_name", display_name.as_str())?;
    }
    if let Some(mission_id) = mission.mission_id {
        table.set("mission_id", mission_id)?;
    }
    if let Some(linkdata_id) = mission.linkdata_id {
        table.set("linkdata_id", linkdata_id)?;
    }
    table.set("aliases", string_array(lua, &mission.aliases)?)?;
    table.set("modes", string_array(lua, &mission.modes)?)?;
    Ok(table)
}

fn string_array(lua: &Lua, values: &[String]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, value) in values.iter().enumerate() {
        table.set(index + 1, value.as_str())?;
    }
    Ok(table)
}

fn values_to_lua_array(lua: &Lua, values: &[JsonValue]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, value) in values.iter().enumerate() {
        table.set(index + 1, json_to_lua(lua, value)?)?;
    }
    Ok(table)
}

fn json_to_lua(lua: &Lua, value: &JsonValue) -> mlua::Result<Value> {
    match value {
        JsonValue::Null => Ok(Value::Nil),
        JsonValue::Bool(value) => Ok(Value::Boolean(*value)),
        JsonValue::Number(value) => json_number_to_lua(value),
        JsonValue::String(value) => Ok(Value::String(lua.create_string(value)?)),
        JsonValue::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, json_to_lua(lua, value)?)?;
            }
            Ok(Value::Table(table))
        }
        JsonValue::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.set(key.as_str(), json_to_lua(lua, value)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

fn json_number_to_lua(value: &serde_json::Number) -> mlua::Result<Value> {
    if let Some(value) = value.as_i64() {
        Ok(Value::Integer(value))
    } else if let Some(value) = value.as_u64() {
        if value <= i64::MAX as u64 {
            Ok(Value::Integer(value as i64))
        } else {
            Ok(Value::Number(value as f64))
        }
    } else if let Some(value) = value.as_f64() {
        Ok(Value::Number(value))
    } else {
        Ok(Value::Nil)
    }
}
