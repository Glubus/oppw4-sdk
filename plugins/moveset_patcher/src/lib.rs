use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
};

use plugin_sdk::{export_plugin, HostApi, Plugin, PluginContext, PluginError, PluginResult};

mod constants;
mod log;
mod state;

struct MovesetPatcher;

impl Plugin for MovesetPatcher {
    const ID: &'static str = constants::PLUGIN_ID;

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        initialize(context.host()).map_err(PluginError::from)
    }
}

export_plugin!(MovesetPatcher);

fn initialize(host: HostApi<'_>) -> Result<(), String> {
    log::init(host);
    state::initialize(host)?;
    host.registry()
        .register_module_descriptor_with_schema(
            constants::PLUGIN_ID,
            "moveset.patch",
            ptr::null_mut(),
            noop_module_install,
            Some(MOVESET_PATCH_SCHEMA_JSON),
            Some(moveset_patch_invoke),
        )
        .map_err(|error| error.to_string())?;
    let edits = state::edit_count();
    log::write(
        host,
        format!("moveset_patcher initialized entry_patches={edits}"),
    );
    Ok(())
}

const MOVESET_PATCH_SCHEMA_JSON: &str = r#"{
  "namespace": "moveset",
  "import_name": "patch",
  "constructible": false,
  "functions": [
    {
      "name": "replace",
      "params": [
        { "name": "character", "type_ref": { "kind": "json" } },
        { "name": "payload", "type_ref": { "kind": "json" } }
      ],
      "returns": { "kind": "json" }
    }
  ],
  "types": [],
  "extensions": [
    {
      "target_type": "sdk.Character",
      "methods": [
        {
          "name": "replace_movesets",
          "function": "replace",
          "returns": { "kind": "json" }
        }
      ]
    }
  ]
}"#;

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

unsafe extern "system" fn moveset_patch_invoke(
    _module_context: *mut c_void,
    function_name_utf8: *const c_char,
    args_json: *const u8,
    args_json_len: usize,
    out_json: *mut u8,
    out_json_len: *mut usize,
) -> i32 {
    let Some(function_name) = cstr_to_str(function_name_utf8) else {
        return -40;
    };
    let Some(args_json) = bytes_to_str(args_json, args_json_len) else {
        return -41;
    };
    match state::replace_from_registry(function_name, args_json) {
        Ok(json) => write_invoke_output(json.as_bytes(), out_json, out_json_len),
        Err(code) => code,
    }
}

unsafe fn cstr_to_str<'a>(value: *const c_char) -> Option<&'a str> {
    (!value.is_null()).then(|| CStr::from_ptr(value).to_str().ok())?
}

unsafe fn bytes_to_str<'a>(bytes: *const u8, len: usize) -> Option<&'a str> {
    if bytes.is_null() && len != 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).ok()
}

unsafe fn write_invoke_output(bytes: &[u8], out_json: *mut u8, out_json_len: *mut usize) -> i32 {
    let Some(out_len) = out_json_len.as_mut() else {
        return -45;
    };
    if out_json.is_null() {
        *out_len = bytes.len();
        return 0;
    }
    if *out_len < bytes.len() {
        *out_len = bytes.len();
        return -46;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_json, bytes.len());
    *out_len = bytes.len();
    0
}
