pub(super) use std::fs;
pub(super) use std::sync::atomic::Ordering;
use std::{
    ffi::c_void,
    sync::{atomic::AtomicUsize, Arc},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) use sdk_bridge::{
    BridgeLoadRequest, BridgeModSource, BridgeRegistry, EventEnvelope, EventKey, ModId,
    ModLifecycle, RegistryEventDescriptor, RegistryFunctionDescriptor, RegistryMethodDescriptor,
    RegistryModuleLoad, RegistryModuleSchema, RegistryMutationDescriptor, RegistryTypeDescriptor,
    RegistryTypeExtensionDescriptor, RegistryTypeRef,
};

pub(super) use crate::register_js_bridge;
use crate::JsModule;

pub(super) static REGISTER_CALLS: AtomicUsize = AtomicUsize::new(0);
pub(super) static REGISTER_MASK: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn counted_register(module_context: *mut c_void, _js: *mut c_void) -> i32 {
    REGISTER_CALLS.fetch_add(1, Ordering::SeqCst);
    REGISTER_MASK.fetch_or(module_context as usize, Ordering::SeqCst);
    0
}

unsafe extern "system" fn noop_register(_module_context: *mut c_void, _js: *mut c_void) -> i32 {
    0
}

pub(super) fn counted_module(
    plugin_id: &str,
    module_name: &str,
    context: usize,
    load: RegistryModuleLoad,
) -> JsModule {
    JsModule {
        plugin_id: plugin_id.to_string(),
        module_name: module_name.to_string(),
        context,
        register: counted_register,
        load,
        schema: None,
        invoke: None,
    }
}

pub(super) fn metadata_module(
    plugin_id: &str,
    module_name: &str,
    load: RegistryModuleLoad,
) -> JsModule {
    JsModule {
        plugin_id: plugin_id.to_string(),
        module_name: module_name.to_string(),
        context: 0,
        register: noop_register,
        load,
        schema: None,
        invoke: None,
    }
}

pub(super) fn schema_module(
    plugin_id: &str,
    module_name: &str,
    schema: RegistryModuleSchema,
    load: RegistryModuleLoad,
) -> JsModule {
    schema_module_with_invoke(plugin_id, module_name, schema, load, character_invoke)
}

pub(super) fn schema_module_with_invoke(
    plugin_id: &str,
    module_name: &str,
    schema: RegistryModuleSchema,
    load: RegistryModuleLoad,
    invoke: fn(&str, &str) -> Result<String, String>,
) -> JsModule {
    JsModule {
        plugin_id: plugin_id.to_string(),
        module_name: module_name.to_string(),
        context: 0,
        register: noop_register,
        load,
        schema: Some(schema),
        invoke: Some(Arc::new(invoke)),
    }
}

fn character_invoke(function_name: &str, args_json: &str) -> Result<String, String> {
    if function_name != "find" {
        return Err(format!("unsupported function: {function_name}"));
    }
    let args = serde_json::from_str::<Vec<serde_json::Value>>(args_json)
        .map_err(|error| format!("bad args json: {error}"))?;
    let id = args
        .first()
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "character.find expects id string".to_string())?;
    if id == "zoro" {
        Ok(serde_json::json!({
            "id": "zoro",
            "name": "Roronoa Zoro",
            "movesetLinkdataEntry": 69,
        })
        .to_string())
    } else {
        Ok("null".to_string())
    }
}

pub(super) fn moveset_invoke(function_name: &str, args_json: &str) -> Result<String, String> {
    if function_name != "replace" {
        return Err(format!("unsupported function: {function_name}"));
    }
    let args = serde_json::from_str::<Vec<serde_json::Value>>(args_json)
        .map_err(|error| format!("bad args json: {error}"))?;
    let character = args
        .first()
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "moveset.replace expects character object".to_string())?;
    let entry = character
        .get("movesetLinkdataEntry")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "missing movesetLinkdataEntry".to_string())?;
    let payload_file = args
        .get(1)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing payload file".to_string())?;
    Ok(serde_json::json!({
        "entry": entry,
        "payloadFile": payload_file,
    })
    .to_string())
}

pub(super) fn character_schema() -> RegistryModuleSchema {
    RegistryModuleSchema::new("sdk", "character")
        .function(
            RegistryFunctionDescriptor::new(
                "find",
                RegistryTypeRef::Optional {
                    inner: Box::new(RegistryTypeRef::Named {
                        name: "Character".to_string(),
                    }),
                },
            )
            .param("id", RegistryTypeRef::String),
        )
        .type_descriptor(
            RegistryTypeDescriptor::new("Character")
                .field("id", RegistryTypeRef::String)
                .field("name", RegistryTypeRef::String)
                .field("movesetLinkdataEntry", RegistryTypeRef::Json),
        )
}

pub(super) fn moveset_schema() -> RegistryModuleSchema {
    RegistryModuleSchema::new("moveset", "patch")
        .function(
            RegistryFunctionDescriptor::new("replace", RegistryTypeRef::Json)
                .param("character", RegistryTypeRef::Json)
                .param("payload", RegistryTypeRef::Json),
        )
        .extension(
            RegistryTypeExtensionDescriptor::new("sdk.Character").method(
                RegistryMethodDescriptor::new("replace_movesets", "replace", RegistryTypeRef::Json),
            ),
        )
}

pub(super) fn player_schema() -> RegistryModuleSchema {
    RegistryModuleSchema::new("sdk", "player").event(RegistryEventDescriptor::new(
        "character_changed",
        "sdk.runtime.player.character_changed",
        RegistryTypeRef::Json,
    ))
}

pub(super) fn runtime_schema_with_mutation() -> RegistryModuleSchema {
    RegistryModuleSchema::new("sdk", "runtime").mutation(RegistryMutationDescriptor::new(
        "apply_fx",
        "sdk.runtime.fx.apply",
        RegistryTypeRef::Json,
    ))
}

pub(super) fn load_js_mod(
    registry: &mut BridgeRegistry,
    mod_id: &str,
    name: &str,
    root: &std::path::Path,
) -> ModLifecycle {
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new(mod_id).expect("mod id"),
            name: name.to_string(),
            source: BridgeModSource::Directory(root.to_path_buf()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load js mod")
}

pub(super) fn temp_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
}

pub(super) fn rust_sources_under(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    collect_rust_sources(root, &mut sources);
    sources
}

fn collect_rust_sources(root: &std::path::Path, sources: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}
