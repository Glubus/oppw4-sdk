use std::ffi::c_void;

use plugin_abi::{
    Oppw4PluginApi, Oppw4RegistryModule, Oppw4RegistryModuleInstallFn, Oppw4RegistryModuleInvokeFn,
};

use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct RegistryService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> RegistryService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn register_module(self, module: &Oppw4RegistryModule) -> PluginResult<()> {
        let register = self
            .abi
            .register_registry_module
            .ok_or(PluginError::MissingHostFunction("register_registry_module"))?;
        let code = r#unsafe::register_registry_module(self.abi.host_context, register, module);
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_registry_module",
                code,
            })
        }
    }

    pub fn register_module_descriptor(
        self,
        plugin_id: &str,
        module_name: &str,
        module_context: *mut c_void,
        install: Oppw4RegistryModuleInstallFn,
    ) -> PluginResult<()> {
        self.register_module_descriptor_with_schema(
            plugin_id,
            module_name,
            module_context,
            install,
            None,
            None,
        )
    }

    pub fn register_module_descriptor_with_schema(
        self,
        plugin_id: &str,
        module_name: &str,
        module_context: *mut c_void,
        install: Oppw4RegistryModuleInstallFn,
        schema_json: Option<&str>,
        invoke: Option<Oppw4RegistryModuleInvokeFn>,
    ) -> PluginResult<()> {
        let plugin_id = cstring_lossy(plugin_id);
        let module_name = cstring_lossy(module_name);
        let schema_json = schema_json.map(cstring_lossy);
        let module = Oppw4RegistryModule {
            plugin_id: plugin_id.as_ptr(),
            module_name: module_name.as_ptr(),
            module_context,
            install: Some(install),
            schema_json: schema_json
                .as_ref()
                .map_or(std::ptr::null(), |schema| schema.as_ptr()),
            invoke,
        };
        self.register_module(&module)
    }
}
