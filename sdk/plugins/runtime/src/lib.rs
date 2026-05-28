mod config;
mod game;
mod mission;
mod reverse;
mod rewards;
mod runtime;

use std::{
    ffi::{c_char, c_void},
    ptr,
};

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        runtime::Runtime::initialize(context.host())?;
        register_player_module(context)?;
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

const PLAYER_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "player",
  "constructible": false,
  "functions": [],
  "types": [],
  "events": [
    {
      "name": "character_changed",
      "key": "sdk.runtime.player.character_changed",
      "payload": { "kind": "json" }
    }
  ]
}"#;

fn register_player_module(context: PluginContext<'_>) -> PluginResult<()> {
    context.register_registry_module_with_schema(
        "sdk.player",
        ptr::null_mut(),
        noop_module_install,
        PLAYER_SCHEMA_JSON,
        player_invoke,
    )
}

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

unsafe extern "system" fn player_invoke(
    _module_context: *mut c_void,
    _function_name_utf8: *const c_char,
    _args_json: *const u8,
    _args_json_len: usize,
    _out_json: *mut u8,
    _out_json_len: *mut usize,
) -> i32 {
    -42
}

export_plugin!(SdkRuntime);
