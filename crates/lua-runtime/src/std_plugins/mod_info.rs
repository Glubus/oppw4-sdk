use mlua::{Lua, Table};

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let module = lua.create_table()?;
    module.set("current", lua.create_function(current)?)?;
    module.set("id", lua.create_function(|lua, ()| mod_id(lua))?)?;
    module.set("name", lua.create_function(|lua, ()| mod_name(lua))?)?;
    module.set("root", lua.create_function(|lua, ()| mod_root(lua))?)?;
    module.set("is_zip", lua.create_function(|lua, ()| is_zip(lua))?)?;
    register_std_module(lua, "mod", module)
}

fn current(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let info = lua.create_table()?;
    info.set("id", mod_id(lua)?)?;
    info.set("name", mod_name(lua)?)?;
    info.set("root", mod_root(lua)?)?;
    info.set("zip_root", zip_root(lua)?)?;
    info.set("is_zip", is_zip(lua)?)?;
    Ok(info)
}

fn mod_id(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_id")
        .map(|value| value.unwrap_or_default())
}

fn mod_name(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_name")
        .map(|value| value.unwrap_or_default())
}

fn mod_root(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_root")
        .map(|value| value.unwrap_or_default())
}

fn zip_root(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_zip_root")
        .map(|value| value.unwrap_or_default())
}

fn is_zip(lua: &Lua) -> mlua::Result<bool> {
    lua.globals()
        .get::<Option<bool>>("__oppw4_mod_is_zip")
        .map(|value| value.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_mod_exposes_current_mod_context() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        lua.globals()
            .set("__oppw4_mod_id", "zoro_moveset")
            .expect("id");
        lua.globals()
            .set("__oppw4_mod_name", "Zoro Moveset")
            .expect("name");
        lua.globals()
            .set("__oppw4_mod_root", r"D:\mods\zoro_moveset.zip")
            .expect("root");
        lua.globals()
            .set("__oppw4_mod_zip_root", "zoro_moveset/")
            .expect("zip root");
        lua.globals()
            .set("__oppw4_mod_is_zip", true)
            .expect("is zip");

        let value: String = lua
            .load(
                r#"
                local mod = require("std.mod")
                local current = mod.current()
                return current.id .. ":" .. current.name .. ":" .. tostring(current.is_zip) .. ":" .. current.zip_root
                "#,
            )
            .eval()
            .expect("mod context");

        assert_eq!(value, "zoro_moveset:Zoro Moveset:true:zoro_moveset/");
    }
}
