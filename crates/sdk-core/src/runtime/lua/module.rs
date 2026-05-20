use std::ffi::c_void;

use mlua::Lua;
use plugin_abi::Oppw4LuaRegisterFn;

#[derive(Clone)]
pub(super) struct RegisteredModule {
    pub(super) plugin_id: String,
    pub(super) module_name: String,
    pub(super) context: usize,
    pub(super) register: Oppw4LuaRegisterFn,
    pub(super) permissions: ModulePermissions,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModulePermissions {
    pub(crate) character_extension: bool,
}

pub(super) fn register_plugin_module(lua: &Lua, module: &RegisteredModule) -> mlua::Result<()> {
    if module.permissions.character_extension {
        lua_api::authorize_character_extension_owner(lua, &module.plugin_id)?;
    }
    let result = unsafe {
        (module.register)(
            module.context as *mut c_void,
            (lua as *const Lua).cast_mut().cast(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(mlua::Error::external(format!(
            "lua module register failed plugin={} module={} result={result}",
            module.plugin_id, module.module_name
        )))
    }
}
