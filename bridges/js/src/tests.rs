use std::{
    ffi::c_void,
    fs,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use sdk_bridge::{
    BridgeLoadRequest, BridgeModSource, BridgeRegistry, EventEnvelope, EventKey, ModId,
    ModLifecycle, RegistryFunctionDescriptor, RegistryModuleLoad, RegistryModuleSchema,
    RegistryTypeDescriptor, RegistryTypeRef,
};

use crate::{register_js_bridge, JsModule};

static REGISTER_CALLS: AtomicUsize = AtomicUsize::new(0);
static REGISTER_MASK: AtomicUsize = AtomicUsize::new(0);

#[test]
fn js_bridge_registers_and_loads_through_rust_registry() {
    let root = temp_root("js-bridge-registry");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(root.join("mod.js"), "const value = 40 + 2;").expect("script");

    let mod_id = ModId::new("ace_moveset").expect("mod id");
    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());

    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id,
            name: "Ace Moveset".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::BootOnce);
    assert_eq!(registry.drain_boot_mutations(), []);
    assert!(registry.drain_load_logs().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn module_selection_comes_from_registry_metadata() {
    REGISTER_CALLS.store(0, Ordering::SeqCst);
    REGISTER_MASK.store(0, Ordering::SeqCst);
    let modules = vec![
        metadata_module("core_api", "core.api", RegistryModuleLoad::Always),
        metadata_module(
            "tool_api",
            "tool.api",
            RegistryModuleLoad::WhenPluginRequested,
        ),
        metadata_module(
            "unused_api",
            "unused.api",
            RegistryModuleLoad::WhenPluginRequested,
        ),
    ];
    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, modules);

    let root = temp_root("js-bridge-module-selection");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(root.join("mod.js"), "").expect("script");

    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("module_selection").expect("mod id"),
            name: "Module Selection".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: vec!["tool_api".to_string()],
        })
        .expect("load mod");

    assert_eq!(REGISTER_CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(REGISTER_MASK.load(Ordering::SeqCst), 1 | 2);
    assert!(registry.drain_boot_mutations().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn dispatch_calls_callback_in_loaded_vm() {
    let root = temp_root("js-bridge-dispatch");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        globalThis.dispatchCount = 0;
        oppw4.events.on("sdk.test.event", (ctx) => {
            if (ctx.eventKey !== "sdk.test.event") {
                throw new Error("bad event key: " + ctx.eventKey);
            }
            if (ctx.payload.value !== 42) {
                throw new Error("bad payload: " + ctx.payloadJson);
            }
            if (ctx.mod.id !== "callback_mod") {
                throw new Error("bad mod metadata: " + ctx.mod.id);
            }
            globalThis.dispatchCount += 1;
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("callback_mod").expect("mod id"),
            name: "Callback Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::EventDriven);
    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.event").expect("event key"),
        payload_json: serde_json::json!({ "value": 42 }).to_string(),
    });

    assert_eq!(report.errors, []);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn dispatch_reports_js_callback_errors() {
    let root = temp_root("js-bridge-dispatch-error");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        oppw4.events.on("sdk.test.event", () => {
            throw new Error("callback exploded");
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("callback_error_mod").expect("mod id"),
            name: "Callback Error Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.event").expect("event key"),
        payload_json: "{}".to_string(),
    });

    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].mod_id.as_str(), "callback_error_mod");
    assert!(report.errors[0].message.contains("js handler call failed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sandbox_hides_quickjs_system_modules() {
    let root = temp_root("js-bridge-sandbox");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        if (globalThis.std !== null) {
            throw new Error("std should be hidden");
        }
        if (globalThis.os !== null) {
            throw new Error("os should be hidden");
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("sandbox_mod").expect("mod id"),
            name: "Sandbox Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::BootOnce);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn registry_api_reads_modules_installed_by_bridge() {
    let root = temp_root("js-bridge-registry-api");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        globalThis.test_module = Object.freeze({ value: 42 });
        if (!oppw4.registry.has("test_module")) {
            throw new Error("registry has() failed");
        }
        if (oppw4.registry.has("missing_module")) {
            throw new Error("registry has() should be false for missing modules");
        }
        const module = oppw4.registry.module("test_module");
        if (!module || module.value !== 42) {
            throw new Error("registry module lookup failed");
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("registry_api_mod").expect("mod id"),
            name: "Registry Api Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::BootOnce);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn registry_api_exposes_selected_module_metadata() {
    let root = temp_root("js-bridge-registry-metadata");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        const modules = oppw4.registry.modules();
        const names = modules.map((module) => module.name).join(",");
        if (names !== "core.api,tool.api") {
            throw new Error("bad registry modules: " + names);
        }
        if (modules[0].providerId !== "core_api" || modules[0].load !== "always") {
            throw new Error("bad always module metadata");
        }
        if (modules[1].providerId !== "tool_api" || modules[1].load !== "when_plugin_requested") {
            throw new Error("bad requested module metadata");
        }
        if (!Object.isFrozen(modules) || !Object.isFrozen(modules[0])) {
            throw new Error("registry metadata should be frozen");
        }
        "#,
    )
    .expect("script");

    let modules = vec![
        counted_module("core_api", "core.api", 1, RegistryModuleLoad::Always),
        counted_module(
            "tool_api",
            "tool.api",
            2,
            RegistryModuleLoad::WhenPluginRequested,
        ),
        counted_module(
            "unused_api",
            "unused.api",
            4,
            RegistryModuleLoad::WhenPluginRequested,
        ),
    ];
    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, modules);
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("registry_metadata_mod").expect("mod id"),
            name: "Registry Metadata Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: vec!["tool_api".to_string()],
        })
        .expect("load mod");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn typed_registry_schema_projects_sdk_namespace_modules() {
    let root = temp_root("js-bridge-typed-registry-schema");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { character } from "sdk";
        if (typeof sdk !== "object" || !sdk.character) {
            throw new Error("sdk.character projection missing");
        }
        if (character !== sdk.character) {
            throw new Error("sdk import should reuse projected registry object");
        }
        if (typeof character.find !== "function") {
            throw new Error("character.find projection missing");
        }
        if (oppw4.registry.module("sdk.character") !== sdk.character) {
            throw new Error("registry path lookup failed");
        }
        if (Object.keys(sdk.character).includes("__schema")) {
            throw new Error("schema should not be enumerable");
        }
        const modules = oppw4.registry.modules();
        const characterModule = modules.find((module) => module.name === "sdk.character");
        if (!characterModule || characterModule.schema.importName !== "character") {
            throw new Error("typed schema metadata missing");
        }
        if (characterModule.schema.constructible !== false) {
            throw new Error("character module should not be constructible");
        }
        if (characterModule.schema.functions[0].name !== "find") {
            throw new Error("character.find descriptor missing");
        }
        try {
            new character();
            throw new Error("character should not be constructible");
        } catch (error) {
            if (String(error).includes("character should not be constructible")) {
                throw error;
            }
        }
        const zoro = character.find("zoro");
        if (!zoro || zoro.id !== "zoro" || zoro.name !== "Roronoa Zoro") {
            throw new Error("bad character result: " + JSON.stringify(zoro));
        }
        const missing = character.find("missing");
        if (missing !== null) {
            throw new Error("missing character should be null");
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![schema_module(
            "sdk_data",
            "sdk.character",
            character_schema(),
            RegistryModuleLoad::Always,
        )],
    );
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("typed_registry_schema_mod").expect("mod id"),
            name: "Typed Registry Schema Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn registry_dispatches_same_event_to_multiple_js_mods() {
    let first_root = temp_root("js-bridge-multi-dispatch-a");
    let second_root = temp_root("js-bridge-multi-dispatch-b");
    fs::create_dir_all(&first_root).expect("first temp dir");
    fs::create_dir_all(&second_root).expect("second temp dir");
    fs::write(
        first_root.join("mod.js"),
        r#"
        oppw4.events.on("sdk.test.broadcast", () => {
            throw new Error("first mod callback reached");
        });
        "#,
    )
    .expect("first script");
    fs::write(
        second_root.join("mod.js"),
        r#"
        oppw4.events.on("sdk.test.broadcast", () => {
            throw new Error("second mod callback reached");
        });
        "#,
    )
    .expect("second script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    load_js_mod(&mut registry, "first_callback_mod", "First", &first_root);
    load_js_mod(&mut registry, "second_callback_mod", "Second", &second_root);

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.broadcast").expect("event key"),
        payload_json: "{}".to_string(),
    });

    assert_eq!(report.errors.len(), 2);
    assert!(report
        .errors
        .iter()
        .any(|error| error.mod_id.as_str() == "first_callback_mod"
            && error.message.contains("js handler call failed")));
    assert!(report
        .errors
        .iter()
        .any(|error| error.mod_id.as_str() == "second_callback_mod"
            && error.message.contains("js handler call failed")));
    let _ = fs::remove_dir_all(first_root);
    let _ = fs::remove_dir_all(second_root);
}

#[test]
fn invalid_json_payload_is_reported_as_js_dispatch_error() {
    let root = temp_root("js-bridge-invalid-json");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        oppw4.events.on("sdk.test.invalid_json", () => {});
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    load_js_mod(&mut registry, "invalid_json_mod", "Invalid Json Mod", &root);

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.invalid_json").expect("event key"),
        payload_json: "{".to_string(),
    });

    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].mod_id.as_str(), "invalid_json_mod");
    assert!(report.errors[0].message.contains("js handler call failed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn bridge_sources_do_not_hardcode_domain_modules() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = [manifest_dir.join("src"), manifest_dir.join("../core/src")];
    let forbidden = [
        concat!("std", ".", "character"),
        concat!("moveset", "_", "patcher"),
        concat!("struct", "_", "api"),
        concat!("character", "_", "extension"),
    ];

    for root in roots {
        for source_file in rust_sources_under(&root) {
            if source_file.ends_with("tests.rs") {
                continue;
            }
            let source = fs::read_to_string(&source_file).expect("source file");
            for token in forbidden {
                assert!(
                    !source.contains(token),
                    "domain token {token:?} found in {}",
                    source_file.display()
                );
            }
        }
    }
}

unsafe extern "system" fn counted_register(module_context: *mut c_void, _js: *mut c_void) -> i32 {
    REGISTER_CALLS.fetch_add(1, Ordering::SeqCst);
    REGISTER_MASK.fetch_or(module_context as usize, Ordering::SeqCst);
    0
}

unsafe extern "system" fn noop_register(_module_context: *mut c_void, _js: *mut c_void) -> i32 {
    0
}

fn counted_module(
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

fn metadata_module(plugin_id: &str, module_name: &str, load: RegistryModuleLoad) -> JsModule {
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

fn schema_module(
    plugin_id: &str,
    module_name: &str,
    schema: RegistryModuleSchema,
    load: RegistryModuleLoad,
) -> JsModule {
    JsModule {
        plugin_id: plugin_id.to_string(),
        module_name: module_name.to_string(),
        context: 0,
        register: noop_register,
        load,
        schema: Some(schema),
        invoke: Some(Arc::new(character_invoke)),
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
        })
        .to_string())
    } else {
        Ok("null".to_string())
    }
}

fn character_schema() -> RegistryModuleSchema {
    RegistryModuleSchema::new("sdk", "character")
        .function(
            RegistryFunctionDescriptor::new(
                "find",
                RegistryTypeRef::Optional(Box::new(RegistryTypeRef::Named(
                    "Character".to_string(),
                ))),
            )
            .param("id", RegistryTypeRef::String),
        )
        .type_descriptor(
            RegistryTypeDescriptor::new("Character")
                .field("id", RegistryTypeRef::String)
                .field("name", RegistryTypeRef::String),
        )
}

fn load_js_mod(
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

fn temp_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
}

fn rust_sources_under(root: &std::path::Path) -> Vec<std::path::PathBuf> {
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
