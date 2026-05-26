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
