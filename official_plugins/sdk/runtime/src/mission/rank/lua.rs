use mlua::{Lua, Table, Value};
use plugin_sdk::{RankCapEffect, RankCapRule, RankCondition, RankConditionExpr};

pub(super) const MODULE_NAME: &str = "sdk.runtime.ranks";

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("slot", lua.create_function(slot)?)?;
    table.set("all", lua.create_function(all)?)?;
    table.set("any", lua.create_function(any)?)?;
    table.set("active_character", lua.create_function(active_character)?)?;
    table.set("flag", lua.create_function(flag)?)?;
    table.set("equals", lua.create_function(equals)?)?;
    table.set("custom", lua.create_function(custom)?)?;
    Ok(table)
}

fn slot(lua: &Lua, slots: Value) -> mlua::Result<Table> {
    let rule = lua.create_table()?;
    rule.set("__rank_slots", parse_slots(slots)?)?;
    rule.set("__rank_condition", Value::Nil)?;

    rule.set(
        "condition",
        lua.create_function(|_, (this, condition): (Table, Value)| {
            this.set("__rank_condition", condition)?;
            Ok(this)
        })?,
    )?;
    rule.set(
        "enable",
        lua.create_function(|_, this: Table| build_rule(this, RankCapEffect::Enable))?,
    )?;
    rule.set(
        "disable",
        lua.create_function(|_, this: Table| build_rule(this, RankCapEffect::Disable))?,
    )?;
    Ok(rule)
}

fn all(lua: &Lua, values: mlua::MultiValue) -> mlua::Result<Table> {
    condition_expr(lua, "all", values)
}

fn any(lua: &Lua, values: mlua::MultiValue) -> mlua::Result<Table> {
    condition_expr(lua, "any", values)
}

fn active_character(lua: &Lua, id: String) -> mlua::Result<Table> {
    condition_table(lua, "active_character", "id", id)
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
    condition_table(lua, "custom", "key", key).and_then(|table| {
        table.set("value", value)?;
        Ok(table)
    })
}

fn condition_table(
    lua: &Lua,
    kind: &'static str,
    key_name: &'static str,
    value: String,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", kind)?;
    table.set(key_name, value)?;
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

fn parse_slots(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::Integer(slot) => Ok(vec![slot.to_string()]),
        Value::String(slot) => Ok(vec![slot.to_str()?.to_string()]),
        Value::Table(table) => table.sequence_values::<Value>().map(parse_slot).collect(),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot or rank slot list".to_string(),
            message: None,
        }),
    }
}

fn parse_slot(value: mlua::Result<Value>) -> mlua::Result<String> {
    match value? {
        Value::Integer(slot) => Ok(slot.to_string()),
        Value::String(slot) => Ok(slot.to_str()?.to_string()),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot".to_string(),
            message: None,
        }),
    }
}

fn build_rule(this: Table, effect: RankCapEffect) -> mlua::Result<String> {
    let slots: Vec<String> = this.get("__rank_slots")?;
    let condition: Value = this.get("__rank_condition")?;
    let rule = RankCapRule::new(effect)
        .slots(slots)
        .condition(parse_condition_expr(condition)?);
    serde_json::to_string(&rule).map_err(mlua::Error::external)
}

fn parse_condition_expr(value: Value) -> mlua::Result<RankConditionExpr> {
    match value {
        Value::Nil => Ok(RankConditionExpr::None),
        Value::Table(table) => match table.get::<Option<String>>("mode")?.as_deref() {
            Some("all") => Ok(RankConditionExpr::All(parse_condition_list(table)?)),
            Some("any") => Ok(RankConditionExpr::Any(parse_condition_list(table)?)),
            _ => Ok(RankConditionExpr::All(vec![parse_condition(table)?])),
        },
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank condition".to_string(),
            message: None,
        }),
    }
}

fn parse_condition_list(table: Table) -> mlua::Result<Vec<RankCondition>> {
    let conditions: Table = table.get("conditions")?;
    conditions
        .sequence_values::<Table>()
        .map(|condition| parse_condition(condition?))
        .collect()
}

fn parse_condition(table: Table) -> mlua::Result<RankCondition> {
    match table.get::<String>("kind")?.as_str() {
        "always" => Ok(RankCondition::Always),
        "flag" => Ok(RankCondition::flag(
            table.get::<String>("key")?,
            table.get::<bool>("value")?,
        )),
        "equals" => Ok(RankCondition::equals(
            table.get::<String>("key")?,
            table.get::<String>("value")?,
        )),
        "active_character" => Ok(RankCondition::active_character(table.get::<String>("id")?)),
        "custom" => Ok(RankCondition::custom(
            table.get::<String>("key")?,
            table.get::<String>("value")?,
        )),
        kind => Err(mlua::Error::external(format!(
            "unknown rank condition kind: {kind}"
        ))),
    }
}
