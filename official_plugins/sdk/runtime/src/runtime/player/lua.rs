use mlua::{Lua, Table, Value};

pub(super) const MODULE_NAME: &str = "sdk.runtime.player";

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("active_character", lua.create_function(active_character)?)?;
    Ok(table)
}

fn active_character(lua: &Lua, args: mlua::MultiValue) -> mlua::Result<Table> {
    match active_character_id(args)? {
        Some(id) => active_character_condition(lua, id),
        None => active_character_builder(lua),
    }
}

fn active_character_builder(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(
        "is",
        lua.create_function(|lua, (_this, id): (Table, String)| {
            active_character_condition(lua, id)
        })?,
    )?;
    Ok(table)
}

fn active_character_condition(lua: &Lua, id: String) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "active_character")?;
    table.set("id", id)?;
    Ok(table)
}

fn active_character_id(args: mlua::MultiValue) -> mlua::Result<Option<String>> {
    let mut values = args.into_iter();
    let first = values.next();
    let value = match first {
        Some(Value::Table(_)) => values.next(),
        other => other,
    };
    match value {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::String(id)) => Ok(Some(id.to_str()?.to_string())),
        Some(other) => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "character id".to_string(),
            message: None,
        }),
    }
}
