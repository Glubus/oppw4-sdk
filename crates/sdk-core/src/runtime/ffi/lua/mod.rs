use plugin_abi::Oppw4LuaModule;

use crate::runtime::lua::{self, ModulePermissions};

mod r#unsafe;

pub(crate) use r#unsafe::host_register_lua_module;

fn register_lua_module(module: *const Oppw4LuaModule, permissions: ModulePermissions) -> i32 {
    unsafe { lua::register_module(module, permissions) }
}
