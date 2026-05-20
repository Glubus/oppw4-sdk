use std::ffi::c_void;

use plugin_abi::{optional_cstr, Oppw4LuaModule};

use crate::runtime::ffi::context::context_from_raw;
use crate::runtime::lua::ModulePermissions;

pub(crate) unsafe extern "system" fn host_register_lua_module(
    host_context: *mut c_void,
    module: *const Oppw4LuaModule,
) -> i32 {
    let Some(module_ref) = module.as_ref() else {
        return -1;
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) = context.require_lua_module_registration(
        optional_cstr(module_ref.plugin_id),
        optional_cstr(module_ref.module_name),
    ) {
        return code;
    }
    super::register_lua_module(
        module,
        ModulePermissions {
            character_extension: context.allows_character_extension(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, ptr};

    use crate::runtime::ffi::ApiContext;

    use super::*;

    unsafe extern "system" fn register_stub(
        _module_context: *mut c_void,
        _lua_context: *mut c_void,
    ) -> i32 {
        0
    }

    #[test]
    fn register_lua_module_rejects_undeclared_manifest_module() {
        let mut context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );
        let plugin_id = CString::new("skin_patcher").expect("static plugin id");
        let module_name = CString::new("shared").expect("static module name");
        let module = Oppw4LuaModule {
            plugin_id: plugin_id.as_ptr(),
            module_name: module_name.as_ptr(),
            module_context: ptr::null_mut(),
            register: Some(register_stub),
        };

        let result =
            unsafe { host_register_lua_module((&mut context as *mut ApiContext).cast(), &module) };

        assert_eq!(result, -24);
    }

    #[test]
    fn register_lua_module_rejects_missing_manifest_capability() {
        let mut context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            ["skin_patcher".to_string()],
        );
        let plugin_id = CString::new("skin_patcher").expect("static plugin id");
        let module_name = CString::new("skin_patcher").expect("static module name");
        let module = Oppw4LuaModule {
            plugin_id: plugin_id.as_ptr(),
            module_name: module_name.as_ptr(),
            module_context: ptr::null_mut(),
            register: Some(register_stub),
        };

        let result =
            unsafe { host_register_lua_module((&mut context as *mut ApiContext).cast(), &module) };

        assert_eq!(result, -22);
    }
}
