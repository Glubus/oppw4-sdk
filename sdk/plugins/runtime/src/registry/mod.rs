mod mission;
mod player;
mod schemas;

use std::{ffi::c_void, ptr};

use plugin_sdk::{PluginContext, PluginResult};

struct RuntimeModule {
    name: &'static str,
    schema_json: &'static str,
    invoke: Option<plugin_sdk::Oppw4RegistryModuleInvokeFn>,
}

const RUNTIME_MODULES: &[RuntimeModule] = &[
    RuntimeModule {
        name: "sdk.player",
        schema_json: schemas::PLAYER_SCHEMA_JSON,
        invoke: Some(player::invoke),
    },
    RuntimeModule {
        name: "sdk.difficulty",
        schema_json: schemas::DIFFICULTY_SCHEMA_JSON,
        invoke: None,
    },
    RuntimeModule {
        name: "sdk.rank",
        schema_json: schemas::RANK_SCHEMA_JSON,
        invoke: None,
    },
    RuntimeModule {
        name: "sdk.rewards",
        schema_json: schemas::REWARDS_SCHEMA_JSON,
        invoke: None,
    },
    RuntimeModule {
        name: "sdk.mission",
        schema_json: schemas::MISSION_SCHEMA_JSON,
        invoke: Some(mission::invoke),
    },
];

pub(crate) fn register_runtime_modules(context: PluginContext<'_>) -> PluginResult<()> {
    for module in RUNTIME_MODULES {
        validate_runtime_module_schema(module)?;
        register_runtime_module(context, module)?;
    }
    Ok(())
}

fn validate_runtime_module_schema(module: &RuntimeModule) -> PluginResult<()> {
    let schema = serde_json::from_str::<sdk_bridge::RegistryModuleSchema>(module.schema_json)
        .map_err(|error| format!("{} schema is invalid: {error}", module.name))?;
    schema
        .validate_contract()
        .map_err(|error| format!("{} schema contract is invalid: {error}", module.name))?;
    Ok(())
}

fn register_runtime_module(context: PluginContext<'_>, module: &RuntimeModule) -> PluginResult<()> {
    context.register_registry_module_with_schema_and_optional_invoke(
        module.name,
        ptr::null_mut(),
        noop_module_install,
        module.schema_json,
        module.invoke,
    )
}

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use sdk_bridge::RegistryModuleSchema;

    use super::RUNTIME_MODULES;

    #[test]
    fn runtime_module_schemas_are_valid_registry_contracts() {
        for module in RUNTIME_MODULES {
            let schema = serde_json::from_str::<RegistryModuleSchema>(module.schema_json)
                .unwrap_or_else(|error| panic!("{} schema is invalid: {error}", module.name));
            schema.validate_contract().unwrap_or_else(|error| {
                panic!("{} schema contract is invalid: {error}", module.name)
            });
        }
    }
}
