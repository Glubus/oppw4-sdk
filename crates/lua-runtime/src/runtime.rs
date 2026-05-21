use mlua::Lua;

mod require;
mod runner;
pub(crate) mod sandbox;

pub(crate) use require::register_std_module;
pub use require::{install_require_hook, register_module};
pub use runner::{run_lua_mod, LuaLogEntry, LuaRunError, LuaRunReport};

pub fn authorize_character_extension_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    crate::std_plugins::authorize_character_extension_owner(lua, owner)
}

pub fn install_runtime(lua: &Lua) -> mlua::Result<()> {
    crate::std_plugins::install(lua)?;
    install_require_hook(lua)
}
