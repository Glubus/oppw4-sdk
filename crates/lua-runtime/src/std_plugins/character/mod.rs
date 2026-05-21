use mlua::{Lua, Table, Value};

use crate::runtime::{register_module, register_std_module};

mod extensions;
mod handles;

#[cfg(test)]
mod tests;

use handles::{
    character_handle_table, custom_character_handle_table, local_player_handle_table,
    unsafe_character_handle_table,
};

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    extensions::install_registry(lua)?;

    let character = lua.create_table()?;
    character.set(
        "find",
        lua.create_function(|lua, query: Value| match query {
            Value::String(name) => {
                let Some(character) = struct_api::find(name.to_str()?.as_ref()) else {
                    return Ok(Value::Nil);
                };
                Ok(Value::Table(character_handle_table(lua, character)?))
            }
            Value::Integer(id) if (0..=u16::MAX as i64).contains(&id) => {
                let Some(character) = struct_api::find_by_id(id as u16) else {
                    return Ok(Value::Nil);
                };
                Ok(Value::Table(character_handle_table(lua, character)?))
            }
            _ => Ok(Value::Nil),
        })?,
    )?;
    character.set(
        "unsafe_find",
        lua.create_function(|lua, query: Value| unsafe_character_handle_table(lua, query))?,
    )?;
    character.set(
        "new",
        lua.create_function(|lua, fields: Table| custom_character_handle_table(lua, fields))?,
    )?;
    character.set(
        "all",
        lua.create_function(|lua, ()| {
            let rows = lua.create_table()?;
            for (index, character) in struct_api::all().iter().enumerate() {
                rows.set(index + 1, character_handle_table(lua, character)?)?;
            }
            Ok(rows)
        })?,
    )?;
    character.set(
        "local_player",
        lua.create_function(|lua, ()| local_player_handle_table(lua))?,
    )?;
    register_std_module(lua, "character", character.clone())?;
    register_module(lua, "character", character.clone())?;
    lua.globals().set("character", character)
}

pub(crate) fn authorize_extension_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    extensions::authorize_owner(lua, owner)
}
