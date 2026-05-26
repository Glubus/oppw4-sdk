use mlua::Lua;

mod require;
mod runner;
pub(crate) mod sandbox;

pub(crate) use require::register_std_module;
pub use require::{install_require_hook, register_module};
pub use runner::{run_lua_mod, LuaLogEntry, LuaRunError, LuaRunReport};

const CHARACTER_AUTHORIZED_OWNERS_TABLE: &str = "__struct_api_authorized_method_owners";

pub fn authorize_character_extension_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    let authorized: mlua::Table = lua.globals().get(CHARACTER_AUTHORIZED_OWNERS_TABLE)?;
    authorized.set(owner.to_ascii_lowercase(), true)
}

pub fn install_runtime(lua: &Lua) -> mlua::Result<()> {
    crate::std_plugins::install(lua)?;
    install_require_hook(lua)
}
