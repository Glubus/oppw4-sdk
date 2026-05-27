use mlua::{Lua, LuaOptions, StdLib, Value};

pub(super) fn new_lua() -> mlua::Result<Lua> {
    Lua::new_with(sandbox_libs(), LuaOptions::default())
}

pub(super) fn hide_unsafe_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in ["debug", "io", "os", "package"] {
        globals.set(name, Value::Nil)?;
    }
    Ok(())
}

pub(super) fn install_mod_globals(lua: &Lua, mod_entry: &lua_api::LuaMod) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("__oppw4_mod_id", mod_entry.manifest.id.as_str())?;
    globals.set("__oppw4_mod_name", mod_entry.manifest.name.as_str())?;
    globals.set(
        "__oppw4_mod_root",
        match &mod_entry.source {
            lua_api::ModSource::Directory(root) => root.to_string_lossy().to_string(),
            lua_api::ModSource::Zip { path, .. } => path.to_string_lossy().to_string(),
        },
    )?;
    globals.set(
        "__oppw4_mod_zip_root",
        match &mod_entry.source {
            lua_api::ModSource::Directory(_) => String::new(),
            lua_api::ModSource::Zip { root, .. } => root.clone(),
        },
    )?;
    globals.set(
        "__oppw4_mod_is_zip",
        matches!(mod_entry.source, lua_api::ModSource::Zip { .. }),
    )
}

fn sandbox_libs() -> StdLib {
    StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH | StdLib::PACKAGE
}
