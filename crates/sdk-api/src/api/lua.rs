use std::ffi::c_void;

use plugin_abi::{Oppw4LuaModule, Oppw4LuaRegisterFn, Oppw4PluginApi};

use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct LuaService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> LuaService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn register_module(self, module: &Oppw4LuaModule) -> PluginResult<()> {
        let register = self
            .abi
            .register_lua_module
            .ok_or(PluginError::MissingHostFunction("register_lua_module"))?;
        let code = r#unsafe::register_lua_module(self.abi.host_context, register, module);
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_lua_module",
                code,
            })
        }
    }

    pub fn register_module_fn(
        self,
        plugin_id: &str,
        module_name: &str,
        module_context: *mut c_void,
        register: Oppw4LuaRegisterFn,
    ) -> PluginResult<()> {
        let plugin_id = cstring_lossy(plugin_id);
        let module_name = cstring_lossy(module_name);
        let module = Oppw4LuaModule {
            plugin_id: plugin_id.as_ptr(),
            module_name: module_name.as_ptr(),
            module_context,
            register: Some(register),
        };
        self.register_module(&module)
    }
}
