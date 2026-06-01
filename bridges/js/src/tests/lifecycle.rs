use super::support::*;

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
        counted_module("core_api", "core.api", 8, RegistryModuleLoad::Always),
        counted_module(
            "tool_api",
            "tool.api",
            16,
            RegistryModuleLoad::WhenPluginRequested,
        ),
        counted_module(
            "unused_api",
            "unused.api",
            32,
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

    let register_mask = REGISTER_MASK.load(Ordering::SeqCst);
    assert_eq!(register_mask & (8 | 16), 8 | 16);
    assert_eq!(register_mask & 32, 0);
    assert!(registry.drain_boot_mutations().is_empty());
    let _ = fs::remove_dir_all(root);
}
