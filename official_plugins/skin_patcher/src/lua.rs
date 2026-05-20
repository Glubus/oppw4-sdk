use std::ffi::c_void;

use mlua::{Function, Lua, Table};
use plugin_sdk::{HostApi, PluginError};

use crate::log;

pub fn register(host: HostApi<'_>) {
    let result = match host.lua().register_module_fn(
        "skin_patcher",
        "skin_patcher",
        std::ptr::null_mut(),
        register_skin_patcher_module,
    ) {
        Ok(()) => 0,
        Err(PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };
    if result != 0 {
        log::write_line(format!(
            "skin_patcher lua module register failed result={result}"
        ));
    }
}

unsafe extern "system" fn register_skin_patcher_module(
    _context: *mut c_void,
    lua: *mut c_void,
) -> i32 {
    let Some(lua) = lua.cast::<Lua>().as_ref() else {
        return -1;
    };
    match skin_patcher_module(lua)
        .and_then(|table| lua_api::register_module(lua, "skin_patcher", table))
    {
        Ok(()) => 0,
        Err(error) => {
            log::write_line(format!("skin_patcher lua module failed: {error}"));
            -2
        }
    }
}

fn skin_patcher_module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", "skin_patcher")?;
    table.set(
        "__oppw4_on_import",
        lua.create_function(|lua, ()| register_character_extensions(lua))?,
    )?;
    Ok(table)
}

fn register_character_extensions(lua: &Lua) -> mlua::Result<()> {
    let register: Function = lua.globals().get("__oppw4_register_character_method")?;
    register.call::<()>((
        "skin_patcher",
        "replace_costume",
        lua.create_function(replace_costume)?,
    ))?;
    register.call::<()>((
        "skin_patcher",
        "replace_portrait",
        lua.create_function(replace_portrait)?,
    ))
}

fn replace_costume(_: &Lua, (character, slot, model): (Table, u16, String)) -> mlua::Result<()> {
    if slot == 0 {
        return Err(mlua::Error::external(
            "replace_costume slot is 1-based; use 1 for the first costume",
        ));
    }
    let name = character_name(&character)?;
    let model_id = character.get::<u16>("model_id").ok();
    log::write_line(format!(
        "lua skin_patcher replace_costume character={name} model_id={model_id:?} slot={slot} model={model}"
    ));
    Ok(())
}

fn replace_portrait(
    _: &Lua,
    (character, slot, portrait): (Table, u16, String),
) -> mlua::Result<()> {
    if slot == 0 {
        return Err(mlua::Error::external(
            "replace_portrait slot is 1-based; use 1 for the first portrait",
        ));
    }
    let name = character_name(&character)?;
    let model_id = character.get::<u16>("model_id").ok();
    log::write_line(format!(
        "lua skin_patcher replace_portrait character={name} model_id={model_id:?} slot={slot} portrait={portrait}"
    ));
    Ok(())
}

fn character_name(character: &Table) -> mlua::Result<String> {
    character
        .get::<Option<String>>("canonical")?
        .or_else(|| character.get::<Option<String>>("name").ok().flatten())
        .ok_or_else(|| mlua::Error::external("skin_patcher method called without a character"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requiring_skin_patcher_adds_character_methods() {
        let lua = Lua::new();
        lua_api::install_runtime(&lua).expect("runtime");
        lua_api::authorize_character_extension_owner(&lua, "skin_patcher").expect("authorize");
        let module = skin_patcher_module(&lua).expect("module");
        lua_api::register_module(&lua, "skin_patcher", module).expect("register");

        let ok: bool = lua
            .load(
                r#"
                local character = require("std.character")
                require("skin_patcher")
                local law = character.find("law")
                law:replace_costume(3, "my_model.g1m")
                law:replace_portrait(2, "portrait.g1t")
                return law.replace_costume ~= nil and law.replace_portrait ~= nil
            "#,
            )
            .eval()
            .expect("eval");

        assert!(ok);
    }
}
