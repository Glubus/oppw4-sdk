use plugin_abi::{optional_cstr, Oppw4RegistryModule};
use sdk_bridge::{RegistryModuleDescriptor, RegistryModuleLoad};

use super::{context_from_raw, CAP_REGISTRY_MODULE};

pub(crate) unsafe extern "system" fn host_register_registry_module(
    host_context: *mut std::ffi::c_void,
    module: *const Oppw4RegistryModule,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) = context.require_capability(CAP_REGISTRY_MODULE) {
        return code;
    }
    let Some(module) = module.as_ref() else {
        return -23;
    };
    let Some(module_name) = optional_cstr(module.module_name) else {
        return -23;
    };
    let module_name = module_name.to_string_lossy().to_string();
    if module_name.is_empty() {
        return -23;
    }
    if let Err(code) = context.require_registry_module(&module_name) {
        return code;
    }
    let Some(install) = module.install else {
        return -23;
    };

    crate::runtime::loader::register_registry_module(RegistryModuleDescriptor {
        provider_id: context.plugin_id().to_string(),
        module_name,
        module_context: module.module_context as usize,
        install: Some(install),
        load: RegistryModuleLoad::WhenPluginRequested,
        schema: None,
    })
}
