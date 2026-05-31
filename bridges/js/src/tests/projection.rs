use super::support::*;

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
fn typed_registry_schema_projects_type_extensions() {
    let root = temp_root("js-bridge-typed-registry-extensions");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { character } from "sdk";
        const zoro = character.find("zoro");
        if (!zoro || typeof zoro.replace_movesets !== "function") {
            throw new Error("character extension method missing");
        }
        if (Object.keys(zoro).includes("replace_movesets")) {
            throw new Error("extension methods should not be enumerable");
        }
        const result = zoro.replace_movesets("zoro_moveset.bin");
        if (!result || result.entry !== 69 || result.payloadFile !== "zoro_moveset.bin") {
            throw new Error("bad extension result: " + JSON.stringify(result));
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![
            schema_module(
                "sdk_data",
                "sdk.character",
                character_schema(),
                RegistryModuleLoad::Always,
            ),
            schema_module_with_invoke(
                "moveset_patcher",
                "moveset.patch",
                moveset_schema(),
                RegistryModuleLoad::WhenPluginRequested,
                moveset_invoke,
            ),
        ],
    );
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("typed_registry_extensions_mod").expect("mod id"),
            name: "Typed Registry Extensions Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: vec!["moveset_patcher".to_string()],
        })
        .expect("load mod");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn typed_registry_schema_projects_event_helpers() {
    let root = temp_root("js-bridge-typed-registry-events");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { player } from "sdk";
        if (typeof player.on_character_changed !== "function") {
            throw new Error("player.on_character_changed projection missing");
        }
        player.on_character_changed((ctx) => {
            if (ctx.eventKey !== "sdk.runtime.player.character_changed") {
                throw new Error("bad event key: " + ctx.eventKey);
            }
            if (!ctx.payload || ctx.payload.characterId !== "whitebeard") {
                throw new Error("bad payload: " + ctx.payloadJson);
            }
            oppw4.trace("typed player event " + ctx.payload.characterId);
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![schema_module(
            "sdk_runtime",
            "sdk.player",
            player_schema(),
            RegistryModuleLoad::Always,
        )],
    );
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("typed_registry_events_mod").expect("mod id"),
            name: "Typed Registry Events Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::EventDriven);
    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.player.character_changed").expect("event key"),
        payload_json: serde_json::json!({ "characterId": "whitebeard" })
            .to_string()
            .into(),
    });

    assert_eq!(report.errors, []);
    assert_eq!(
        report.logs,
        ["js trace mod=typed_registry_events_mod typed player event whitebeard"]
    );
    let _ = fs::remove_dir_all(root);
}
