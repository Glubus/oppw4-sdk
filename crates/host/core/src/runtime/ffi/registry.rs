use std::{ffi::CString, sync::Arc};

use plugin_abi::{optional_cstr, Oppw4RegistryModule, Oppw4RegistryModuleInvokeFn};
use sdk_bridge::{RegistryModuleDescriptor, RegistryModuleLoad, RegistryModuleSchema};

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
    let schema = match registry_schema(module.schema_json) {
        Ok(schema) => schema,
        Err(code) => return code,
    };
    let invoke = module
        .invoke
        .map(|callback| registry_invoke(module.module_context as usize, callback));

    crate::runtime::loader::register_registry_module(RegistryModuleDescriptor {
        provider_id: context.plugin_id().to_string(),
        module_name,
        module_context: module.module_context as usize,
        install: module.install,
        invoke,
        load: RegistryModuleLoad::WhenPluginRequested,
        schema,
    })
}

unsafe fn registry_schema(
    schema_json: *const std::ffi::c_char,
) -> Result<Option<RegistryModuleSchema>, i32> {
    let Some(schema_json) = optional_cstr(schema_json) else {
        return Ok(None);
    };
    let schema_json = schema_json.to_string_lossy();
    if schema_json.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str::<RegistryModuleSchema>(&schema_json)
        .map(Some)
        .map_err(|_| -26)
}

fn registry_invoke(
    module_context: usize,
    callback: Oppw4RegistryModuleInvokeFn,
) -> Arc<dyn Fn(&str, &str) -> Result<String, String> + Send + Sync + 'static> {
    Arc::new(move |function_name, args_json| {
        let function_name = CString::new(function_name)
            .map_err(|_| "function name contains nul byte".to_string())?;
        let args = args_json.as_bytes();
        let mut out = vec![0u8; 64 * 1024];
        let mut written_len = out.len();
        let code = unsafe {
            callback(
                module_context as *mut _,
                function_name.as_ptr(),
                args.as_ptr(),
                args.len(),
                out.as_mut_ptr(),
                &mut written_len,
            )
        };
        if code == -46 && written_len > out.len() {
            out.resize(written_len, 0);
            let retry = unsafe {
                callback(
                    module_context as *mut _,
                    function_name.as_ptr(),
                    args.as_ptr(),
                    args.len(),
                    out.as_mut_ptr(),
                    &mut written_len,
                )
            };
            if retry != 0 {
                return Err(format!("invoke retry failed with code {retry}"));
            }
        } else if code != 0 {
            return Err(format!("invoke failed with code {code}"));
        }
        if written_len > out.len() {
            return Err("invoke wrote beyond output buffer".to_string());
        }
        out.truncate(written_len);
        String::from_utf8(out).map_err(|error| format!("invoke returned invalid utf8: {error}"))
    })
}
