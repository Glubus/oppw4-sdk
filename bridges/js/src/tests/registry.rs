use super::support::*;

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
fn registry_metadata_exposes_mutation_contracts() {
    let root = temp_root("js-bridge-mutation-contracts");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        const modules = oppw4.registry.modules();
        const runtime = modules.find((module) => module.name === "sdk.runtime");
        if (!runtime || !runtime.schema.mutations || runtime.schema.mutations.length !== 1) {
            throw new Error("mutation contracts missing");
        }
        const mutation = runtime.schema.mutations[0];
        if (mutation.name !== "apply_fx" || mutation.key !== "sdk.runtime.fx.apply") {
            throw new Error("bad mutation contract: " + JSON.stringify(mutation));
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![schema_module(
            "sdk_runtime",
            "sdk.runtime",
            runtime_schema_with_mutation(),
            RegistryModuleLoad::Always,
        )],
    );
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("mutation_contracts_mod").expect("mod id"),
            name: "Mutation Contracts Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn namespace_module_exports_are_deduplicated() {
    let root = temp_root("js-bridge-namespace-dedup");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { character } from "sdk";
        if (typeof character.find !== "function") {
            throw new Error("character.find projection missing");
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![
            schema_module(
                "sdk_data_a",
                "sdk.character",
                character_schema(),
                RegistryModuleLoad::Always,
            ),
            schema_module(
                "sdk_data_b",
                "sdk.character",
                character_schema(),
                RegistryModuleLoad::WhenPluginRequested,
            ),
        ],
    );
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("namespace_module_dedup_mod").expect("mod id"),
            name: "Namespace Module Dedup Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    let _ = fs::remove_dir_all(root);
}
