use std::{
    ffi::{c_char, c_void, CStr},
    path::{Path, PathBuf},
};

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

mod log;

struct SdkData;

impl Plugin for SdkData {
    const ID: &'static str = "sdk_data";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        log::init(context.host());
        initialize_data(context);
        register_character_module(context)?;
        Ok(())
    }
}

const CHARACTER_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "character",
  "constructible": false,
  "functions": [
    {
      "name": "find",
      "params": [{ "name": "id", "type_ref": { "kind": "string" } }],
      "returns": {
        "kind": "optional",
        "inner": { "kind": "named", "name": "Character" }
      }
    }
  ],
  "types": [
    {
      "name": "Character",
      "constructible": false,
      "fields": [
        { "name": "id", "type_ref": { "kind": "string" } },
        { "name": "name", "type_ref": { "kind": "string" } },
        { "name": "playableId", "type_ref": { "kind": "json" } },
        { "name": "runtimeId", "type_ref": { "kind": "json" } },
        { "name": "bossRuntimeId", "type_ref": { "kind": "json" } },
        { "name": "modelId", "type_ref": { "kind": "json" } },
        { "name": "movesetLinkdataEntry", "type_ref": { "kind": "json" } }
      ]
    }
  ]
}"#;

fn register_character_module(context: PluginContext<'_>) -> PluginResult<()> {
    context.register_registry_module_with_schema(
        "sdk.character",
        std::ptr::null_mut(),
        noop_module_install,
        CHARACTER_SCHEMA_JSON,
        character_invoke,
    )
}

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

unsafe extern "system" fn character_invoke(
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
    let result = match function_name {
        "find" => character_find(args_json),
        _ => Err(-42),
    };
    match result {
        Ok(json) => write_invoke_output(json.as_bytes(), out_json, out_json_len),
        Err(code) => code,
    }
}

fn character_find(args_json: &str) -> Result<String, i32> {
    let args = serde_json::from_str::<Vec<serde_json::Value>>(args_json).map_err(|_| -43)?;
    let query = args
        .first()
        .and_then(serde_json::Value::as_str)
        .ok_or(-44)?;
    let Some(character) = struct_api::find(query) else {
        return Ok("null".to_string());
    };
    Ok(serde_json::json!({
        "id": character.canonical,
        "name": character.display_name,
        "playableId": character.playable_id,
        "runtimeId": character.runtime_id,
        "bossRuntimeId": character.boss_runtime_id,
        "modelId": character.model_id,
        "movesetLinkdataEntry": character.moveset_linkdata_entry,
        "modelStem": character.model_stem,
        "aliases": character.aliases,
    })
    .to_string())
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

fn initialize_data(context: PluginContext<'_>) {
    let Some(game_root) = context.game_root() else {
        struct_api::mark_data_unavailable();
        context.log("sdk.data unavailable: game root is missing");
        return;
    };
    let data_root = data_root(&game_root);
    match struct_api::initialize_data_root(&data_root) {
        Ok(()) => context.log(format!(
            "sdk.data loaded OPPW4 data from {}",
            data_root.display()
        )),
        Err(error) => {
            struct_api::mark_data_unavailable();
            context.log(format!(
                "sdk.data unavailable at {}: {error:?}",
                data_root.display()
            ));
        }
    }
}

fn data_root(game_root: &Path) -> PathBuf {
    game_root.join("oppw4-data")
}

export_plugin!(SdkData);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_invoke_finds_known_character() {
        let function = std::ffi::CString::new("find").expect("function");
        let args = br#"["zoro"]"#;
        let mut required_len = 0usize;

        let first = unsafe {
            character_invoke(
                std::ptr::null_mut(),
                function.as_ptr(),
                args.as_ptr(),
                args.len(),
                std::ptr::null_mut(),
                &mut required_len,
            )
        };

        assert_eq!(first, 0);
        assert!(required_len > 0);

        let mut out = vec![0u8; required_len];
        let mut written_len = out.len();
        let second = unsafe {
            character_invoke(
                std::ptr::null_mut(),
                function.as_ptr(),
                args.as_ptr(),
                args.len(),
                out.as_mut_ptr(),
                &mut written_len,
            )
        };

        assert_eq!(second, 0);
        out.truncate(written_len);
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value["id"], "zoro");
        assert_eq!(value["movesetLinkdataEntry"], 69);
    }
}
