use mlua::{Lua, Table, Value as LuaValue};
use serde_json::Value as JsonValue;

pub(super) fn value_to_lua(lua: &Lua, value: &JsonValue) -> mlua::Result<LuaValue> {
    match value {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(value) => Ok(LuaValue::Boolean(*value)),
        JsonValue::Number(value) => number_to_lua(value),
        JsonValue::String(value) => Ok(LuaValue::String(lua.create_string(value)?)),
        JsonValue::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, value_to_lua(lua, value)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        JsonValue::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.set(key.as_str(), value_to_lua(lua, value)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

pub(super) fn values_to_lua_array(lua: &Lua, values: &[JsonValue]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, value) in values.iter().enumerate() {
        table.set(index + 1, value_to_lua(lua, value)?)?;
    }
    Ok(table)
}

fn number_to_lua(value: &serde_json::Number) -> mlua::Result<LuaValue> {
    if let Some(value) = value.as_i64() {
        Ok(LuaValue::Integer(value))
    } else if let Some(value) = value.as_u64() {
        if value <= i64::MAX as u64 {
            Ok(LuaValue::Integer(value as i64))
        } else {
            Ok(LuaValue::Number(value as f64))
        }
    } else if let Some(value) = value.as_f64() {
        Ok(LuaValue::Number(value))
    } else {
        Ok(LuaValue::Nil)
    }
}
