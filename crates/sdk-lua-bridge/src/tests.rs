use std::{
    ffi::c_void,
    fs,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use sdk_bridge::{
    BridgeLoadRequest, BridgeModSource, BridgeModuleLoad, BridgeRegistry, EventEnvelope, EventKey,
    ModId, ModLifecycle,
};

use crate::{register_lua_bridge, LuaModule};

static REGISTER_CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn lua_bridge_registers_and_loads_through_rust_registry() {
    let root = temp_root("lua-bridge-registry");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.lua"),
        r#"
        local value = 40 + 2
        "#,
    )
    .expect("script");

    let mod_id = ModId::new("ace_moveset").expect("mod id");
    let mut registry = BridgeRegistry::new();
    register_lua_bridge(&mut registry, Vec::new());

    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: mod_id.clone(),
            name: "Ace Moveset".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.lua".to_string(),
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
    let modules = vec![
        module("core_api", "core.api", BridgeModuleLoad::Always),
        module(
            "tool_api",
            "tool.api",
            BridgeModuleLoad::WhenPluginRequested,
        ),
    ];
    let mut registry = BridgeRegistry::new();
    register_lua_bridge(&mut registry, modules);

    let root = temp_root("lua-bridge-module-selection");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(root.join("mod.lua"), "").expect("script");

    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("module_selection").expect("mod id"),
            name: "Module Selection".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.lua".to_string(),
            uses_plugins: vec!["tool_api".to_string()],
        })
        .expect("load mod");

    assert_eq!(REGISTER_CALLS.load(Ordering::SeqCst), 2);
    assert!(registry.drain_boot_mutations().is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn dispatch_calls_callback_in_loaded_vm() {
    let root = temp_root("lua-bridge-dispatch");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.lua"),
        r#"
        __oppw4_register_handler("sdk.test.event", function(event_key, payload_json)
            if event_key ~= "sdk.test.event" then
                error("bad event key: " .. tostring(event_key))
            end
            if payload_json ~= "{\"value\":42}" then
                error("bad payload: " .. tostring(payload_json))
            end
        end)
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_lua_bridge(&mut registry, Vec::new());
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("callback_mod").expect("mod id"),
            name: "Callback Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.lua".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::EventDriven);
    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.event").expect("event key"),
        payload_json: "{\"value\":42}".to_string(),
    });

    assert_eq!(report.errors, []);
    let _ = fs::remove_dir_all(root);
}

unsafe extern "system" fn noop_register(_module_context: *mut c_void, _lua: *mut c_void) -> i32 {
    REGISTER_CALLS.fetch_add(1, Ordering::SeqCst);
    0
}

fn module(plugin_id: &str, module_name: &str, load: BridgeModuleLoad) -> LuaModule {
    LuaModule {
        plugin_id: plugin_id.to_string(),
        module_name: module_name.to_string(),
        context: 0,
        register: noop_register,
        load,
    }
}

fn temp_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
}
