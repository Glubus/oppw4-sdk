mod lua;

#[cfg(test)]
mod lua_tests;

use crate::runtime::lua_module;

lua_module::runtime_lua_module! {
    type = PlayerLuaModule,
    module = lua::MODULE_NAME,
    factory = lua::module,
}

pub(crate) fn lua_module() -> PlayerLuaModule {
    PlayerLuaModule
}

#[cfg(test)]
pub(crate) fn lua_module_for_test(lua_state: &mlua::Lua) -> mlua::Result<mlua::Table> {
    lua::module(lua_state)
}

#[cfg(test)]
pub(crate) const LUA_MODULE_NAME_FOR_TEST: &str = lua::MODULE_NAME;
