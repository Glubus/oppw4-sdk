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
    }?;
    if module.permissions.character_extension {
        lua_api::authorize_character_extension_owner(lua, &module.plugin_id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Table;
    use std::ptr;

    unsafe extern "system" fn install_character_registry(
        _context: *mut c_void,
        lua: *mut c_void,
    ) -> i32 {
        let Some(lua) = (unsafe { lua.cast::<Lua>().as_ref() }) else {
            return -1;
        };
        let globals = lua.globals();
        let Ok(authorized) = lua.create_table() else {
            return -2;
        };
        if globals
            .set("__struct_api_authorized_method_owners", authorized)
            .is_err()
        {
            return -3;
        }
        0
    }

    #[test]
    fn character_registry_module_can_authorize_after_registration() {
        let lua = Lua::new();
        let module = RegisteredModule {
            plugin_id: "sdk_data".to_string(),
            module_name: "std.character".to_string(),
            context: ptr::null_mut::<c_void>() as usize,
            register: install_character_registry,
            permissions: ModulePermissions {
                character_extension: true,
            },
        };

        register_plugin_module(&lua, &module).expect("register module");

        let authorized: Table = lua
            .globals()
            .get("__struct_api_authorized_method_owners")
            .expect("authorized table");
        assert_eq!(authorized.get::<bool>("sdk_data").expect("owner"), true);
    }
}
