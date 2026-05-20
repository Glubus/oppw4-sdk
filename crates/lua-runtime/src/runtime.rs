use mlua::Lua;

mod require;
mod runner;
mod sandbox;
mod standard;

pub use require::{install_require_hook, register_module};
pub use runner::{run_lua_mod, LuaLogEntry, LuaRunError, LuaRunReport};

pub fn authorize_character_extension_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    standard::authorize_character_extension_owner(lua, owner)
}

pub fn install_runtime(lua: &Lua) -> mlua::Result<()> {
    standard::install(lua)?;
    install_require_hook(lua)
}
