mod mission;
mod player;
mod snapshot;

use std::{ffi::c_void, ptr};

use plugin_sdk::{PluginContext, PluginResult};
use sdk_runtime_schema::runtime_schemas;

struct RuntimeModule {
    name: String,
    schema: sdk_schema::RegistryModuleSchema,
    invoke: Option<plugin_sdk::Oppw4RegistryModuleInvokeFn>,
}

pub(crate) fn register_runtime_modules(context: PluginContext<'_>) -> PluginResult<()> {
    for module in runtime_modules() {
        validate_runtime_module_schema(&module)?;
        register_runtime_module(context, &module)?;
    }
    Ok(())
}

fn validate_runtime_module_schema(module: &RuntimeModule) -> PluginResult<()> {
    module
        .schema
        .validate_contract()
        .map_err(|error| format!("{} schema contract is invalid: {error}", module.name))?;
    Ok(())
}

fn register_runtime_module(context: PluginContext<'_>, module: &RuntimeModule) -> PluginResult<()> {
    context.register_registry_module_with_generated_schema_and_optional_invoke(
        &module.name,
        ptr::null_mut(),
        noop_module_install,
        &module.schema,
        module.invoke,
    )
}

fn runtime_modules() -> Vec<RuntimeModule> {
    runtime_schemas()
        .into_iter()
        .map(|schema| {
            let module_name = format!("{}.{}", schema.namespace, schema.import_name);
            let invoke: Option<plugin_sdk::Oppw4RegistryModuleInvokeFn> = match module_name.as_str()
            {
                "sdk.player" => Some(player::invoke as plugin_sdk::Oppw4RegistryModuleInvokeFn),
                "sdk.mission" => Some(mission::invoke as plugin_sdk::Oppw4RegistryModuleInvokeFn),
                "sdk.snapshot" => Some(snapshot::invoke as plugin_sdk::Oppw4RegistryModuleInvokeFn),
                _ => None,
            };
            RuntimeModule {
                name: module_name,
                schema,
                invoke,
            }
        })
        .collect()
}

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::runtime_modules;

    #[test]
    fn runtime_module_schemas_are_valid_registry_contracts() {
        for module in runtime_modules() {
            module.schema.validate_contract().unwrap_or_else(|error| {
                panic!("{} schema contract is invalid: {error}", module.name)
            });
        }
    }
}
