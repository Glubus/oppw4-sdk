use mlua::{Lua, Table, Value};
use plugin_sdk::{
    DifficultyAction, DifficultyActorStat, DifficultyCondition, DifficultyConditionExpr,
    DifficultyFixedArea, DifficultyKnownTable, DifficultyLevel, DifficultyRule, DifficultyValueOp,
};

pub(super) const MODULE_NAME: &str = "sdk.runtime.difficulty";

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("level", lua.create_function(level)?)?;
    table.set("all", lua.create_function(all)?)?;
    table.set("any", lua.create_function(any)?)?;
    table.set("active_character", lua.create_function(active_character)?)?;
    table.set("flag", lua.create_function(flag)?)?;
    table.set("equals", lua.create_function(equals)?)?;
    table.set("custom", lua.create_function(custom)?)?;
    Ok(table)
}

fn level(lua: &Lua, levels: Value) -> mlua::Result<Table> {
    let rule = lua.create_table()?;
    rule.set("__difficulty_levels", parse_levels(levels)?)?;
    rule.set("__difficulty_condition", Value::Nil)?;
    rule.set(
        "condition",
        lua.create_function(|_, (this, condition): (Table, Value)| {
            this.set("__difficulty_condition", condition)?;
            Ok(this)
        })?,
    )?;
    rule.set(
        "enable",
        lua.create_function(|_, this: Table| build_rule(this, DifficultyAction::EnableLevel))?,
    )?;
    rule.set(
        "disable",
        lua.create_function(|_, this: Table| build_rule(this, DifficultyAction::DisableLevel))?,
    )?;
    rule.set(
        "stat",
        lua.create_function(|lua, (this, stat): (Table, String)| {
            operation_builder(lua, this, move |operation| DifficultyAction::ActorStat {
                stat: DifficultyActorStat::new(stat.clone()),
                operation,
            })
        })?,
    )?;
    rule.set(
        "combat_pressure",
        lua.create_function(|lua, this: Table| {
            operation_builder(lua, this, |operation| DifficultyAction::CombatPressure {
                operation,
            })
        })?,
    )?;
    rule.set(
        "table",
        lua.create_function(|lua, (this, table): (Table, String)| {
            operation_builder(lua, this, move |operation| DifficultyAction::KnownTable {
                table: DifficultyKnownTable::new(table.clone()),
                operation,
            })
        })?,
    )?;
    rule.set(
        "raw",
        lua.create_function(|lua, (this, area, offset): (Table, String, Value)| {
            let offset = parse_offset(offset)?;
            operation_builder(lua, this, move |operation| {
                DifficultyAction::RawFixedTable {
                    area: DifficultyFixedArea::new(area.clone()),
                    offset,
                    operation,
                }
            })
        })?,
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

fn operation_builder<F>(lua: &Lua, rule: Table, action: F) -> mlua::Result<Table>
where
    F: Fn(DifficultyValueOp) -> DifficultyAction + Clone + Send + 'static,
{
    let builder = lua.create_table()?;
    builder.set("__difficulty_rule", rule)?;
    builder.set(
        "set",
        lua.create_function({
            let action = action.clone();
            move |_, (this, value): (Table, Value)| {
                build_operation_rule(this, action(DifficultyValueOp::SetF32(parse_f32(value)?)))
            }
        })?,
    )?;
    builder.set(
        "add",
        lua.create_function({
            let action = action.clone();
            move |_, (this, value): (Table, Value)| {
                build_operation_rule(this, action(DifficultyValueOp::AddF32(parse_f32(value)?)))
            }
        })?,
    )?;
    builder.set(
        "multiply",
        lua.create_function({
            let action = action.clone();
            move |_, (this, value): (Table, Value)| {
                build_operation_rule(this, action(DifficultyValueOp::ScaleF32(parse_f32(value)?)))
            }
        })?,
    )?;
    builder.set(
        "set_u16",
        lua.create_function({
            let action = action.clone();
            move |_, (this, value): (Table, Value)| {
                build_operation_rule(this, action(DifficultyValueOp::SetU16(parse_u16(value)?)))
            }
        })?,
    )?;
    builder.set(
        "add_i16",
        lua.create_function({
            let action = action.clone();
            move |_, (this, value): (Table, Value)| {
                build_operation_rule(this, action(DifficultyValueOp::AddI16(parse_i16(value)?)))
            }
        })?,
    )?;
    builder.set(
        "set_u8",
        lua.create_function(move |_, (this, value): (Table, Value)| {
            build_operation_rule(this, action(DifficultyValueOp::SetU8(parse_u8(value)?)))
        })?,
    )?;
    Ok(builder)
}

fn build_operation_rule(this: Table, action: DifficultyAction) -> mlua::Result<String> {
    let rule: Table = this.get("__difficulty_rule")?;
    build_rule(rule, action)
}

fn build_rule(this: Table, action: DifficultyAction) -> mlua::Result<String> {
    let levels: Vec<String> = this.get("__difficulty_levels")?;
    let condition: Value = this.get("__difficulty_condition")?;
    let rule = DifficultyRule {
        levels: levels.into_iter().map(DifficultyLevel::new).collect(),
        condition: parse_condition_expr(condition)?,
        action,
        enabled: true,
    };
    serde_json::to_string(&rule).map_err(mlua::Error::external)
}

fn parse_levels(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::Integer(level) => Ok(vec![normalize_level(level.to_string())]),
        Value::String(level) => Ok(vec![normalize_level(level.to_str()?.as_ref())]),
        Value::Table(table) => table.sequence_values::<Value>().map(parse_level).collect(),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "difficulty level or difficulty level list".to_string(),
            message: None,
        }),
    }
}

fn parse_level(value: mlua::Result<Value>) -> mlua::Result<String> {
    match value? {
        Value::Integer(level) => Ok(normalize_level(level.to_string())),
        Value::String(level) => Ok(normalize_level(level.to_str()?.as_ref())),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "difficulty level".to_string(),
            message: None,
        }),
    }
}

fn normalize_level(value: impl Into<String>) -> String {
    DifficultyLevel::new(value).as_str().to_string()
}

fn parse_condition_expr(value: Value) -> mlua::Result<DifficultyConditionExpr> {
    match value {
        Value::Nil => Ok(DifficultyConditionExpr::None),
        Value::Table(table) => match table.get::<Option<String>>("mode")?.as_deref() {
            Some("all") => Ok(DifficultyConditionExpr::All(parse_condition_list(table)?)),
            Some("any") => Ok(DifficultyConditionExpr::Any(parse_condition_list(table)?)),
            _ => Ok(DifficultyConditionExpr::All(vec![parse_condition(table)?])),
        },
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "difficulty condition".to_string(),
            message: None,
        }),
    }
}

fn parse_condition_list(table: Table) -> mlua::Result<Vec<DifficultyCondition>> {
    let conditions: Table = table.get("conditions")?;
    conditions
        .sequence_values::<Table>()
        .map(|condition| parse_condition(condition?))
        .collect()
}

fn parse_condition(table: Table) -> mlua::Result<DifficultyCondition> {
    match table.get::<String>("kind")?.as_str() {
        "always" => Ok(DifficultyCondition::Always),
        "flag" => Ok(DifficultyCondition::flag(
            table.get::<String>("key")?,
            table.get::<bool>("value")?,
        )),
        "equals" => Ok(DifficultyCondition::equals(
            table.get::<String>("key")?,
            table.get::<String>("value")?,
        )),
        "active_character" => Ok(DifficultyCondition::active_character(
            table.get::<String>("id")?,
        )),
        "custom" => Ok(DifficultyCondition::custom(
            table.get::<String>("key")?,
            table.get::<String>("value")?,
        )),
        kind => Err(mlua::Error::external(format!(
            "unknown difficulty condition kind: {kind}"
        ))),
    }
}

fn parse_offset(value: Value) -> mlua::Result<usize> {
    match value {
        Value::Integer(value) if value >= 0 => Ok(value as usize),
        Value::String(value) => parse_usize_string(value.to_str()?.as_ref()),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "positive integer or hex offset string".to_string(),
            message: None,
        }),
    }
}

fn parse_f32(value: Value) -> mlua::Result<f32> {
    match value {
        Value::Integer(value) => Ok(value as f32),
        Value::Number(value) => Ok(value as f32),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "number".to_string(),
            message: None,
        }),
    }
}

fn parse_u16(value: Value) -> mlua::Result<u16> {
    u16::try_from(parse_i64(value, "u16")?).map_err(mlua::Error::external)
}

fn parse_i16(value: Value) -> mlua::Result<i16> {
    i16::try_from(parse_i64(value, "i16")?).map_err(mlua::Error::external)
}

fn parse_u8(value: Value) -> mlua::Result<u8> {
    u8::try_from(parse_i64(value, "u8")?).map_err(mlua::Error::external)
}

fn parse_i64(value: Value, type_name: &'static str) -> mlua::Result<i64> {
    match value {
        Value::Integer(value) => Ok(value),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: type_name.to_string(),
            message: None,
        }),
    }
}

fn parse_usize_string(value: &str) -> mlua::Result<usize> {
    let value = value.trim();
    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16)
    } else {
        value.parse()
    };
    parsed.map_err(mlua::Error::external)
}
