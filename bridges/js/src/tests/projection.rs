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
            if (!ctx.current_character || ctx.current_character.id !== "whitebeard") {
                throw new Error("bad payload: " + ctx.payloadJson);
            }
            if (ctx.previous_character !== null) {
                throw new Error("previous_character should be null on first event");
            }
            if (ctx.active_character_ids[0] !== "whitebeard") {
                throw new Error("bad active_character_ids");
            }
            oppw4.trace("typed player event " + ctx.current_character.id);
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![
            schema_module(
                "sdk_runtime",
                "sdk.player",
                player_schema(),
                RegistryModuleLoad::Always,
            ),
            schema_module(
                "sdk_data",
                "sdk.character",
                character_schema(),
                RegistryModuleLoad::Always,
            ),
        ],
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
        payload_json: serde_json::json!({
            "current_character_id": "whitebeard",
            "active_character_ids": ["whitebeard"]
        })
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

#[test]
fn typed_registry_schema_projects_difficulty_and_rank_contexts() {
    let root = temp_root("js-bridge-typed-runtime-contexts");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { difficulty, rank } from "sdk";
        difficulty.on_applied((ctx) => {
            if (ctx.mode !== "legend" || ctx.difficulty !== "hard") {
                throw new Error("bad difficulty context");
            }
        });
        rank.on_result((ctx) => {
            if (ctx.rank.final !== "S+") {
                throw new Error("bad final rank");
            }
            if (ctx.rank.count !== "A" || ctx.rank.time !== "S" || ctx.rank.merge !== "S+") {
                throw new Error("bad rank breakdown");
            }
            if (ctx.mission.mode !== "legend" || ctx.mission.difficulty !== "hard") {
                throw new Error("bad mission context");
            }
            oppw4.trace("typed rank result " + ctx.rank.final);
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![
            schema_module(
                "sdk_runtime",
                "sdk.difficulty",
                difficulty_schema(),
                RegistryModuleLoad::Always,
            ),
            schema_module(
                "sdk_runtime",
                "sdk.rank",
                rank_schema(),
                RegistryModuleLoad::Always,
            ),
        ],
    );
    let lifecycle = load_js_mod(&mut registry, "typed_runtime_contexts_mod", "Typed Runtime Contexts Mod", &root);
    assert_eq!(lifecycle, ModLifecycle::EventDriven);

    let difficulty_report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.difficulty.event").expect("event key"),
        payload_json: serde_json::json!({
            "mission_id": 35,
            "mode": "legend",
            "difficulty": "hard"
        })
        .to_string()
        .into(),
    });
    assert_eq!(difficulty_report.errors, []);

    let rank_report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.rank.event").expect("event key"),
        payload_json: serde_json::json!({
            "rank": "S+",
            "count": "A",
            "time": "S",
            "merge": "S+",
            "mission_id": 35,
            "mode": "legend",
            "difficulty": "hard"
        })
        .to_string()
        .into(),
    });

    assert_eq!(rank_report.errors, []);
    assert_eq!(
        rank_report.logs,
        ["js trace mod=typed_runtime_contexts_mod typed rank result S+"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn typed_registry_schema_projects_rewards_contexts() {
    let root = temp_root("js-bridge-typed-rewards-contexts");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { rewards } from "sdk";
        rewards.on_event((ctx) => {
            if (ctx.rank !== "S+" || ctx.berry !== 321) {
                throw new Error("bad rewards event");
            }
            if (ctx.crew_points !== 180) {
                throw new Error("bad crew_points");
            }
            if (ctx.medals.length !== 1 || ctx.medals[0].item_id !== 77) {
                throw new Error("bad reward medals");
            }
            if (ctx.ranks.join(",") !== "A,S,S+,S+") {
                throw new Error("bad reward ranks");
            }
        });
        rewards.on_medals((ctx) => {
            if (ctx.entries.length !== 1 || ctx.entries[0].item_id !== 77) {
                throw new Error("bad rewards medals");
            }
            oppw4.trace("typed rewards medals " + ctx.entries[0].item_id);
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![schema_module(
            "sdk_runtime",
            "sdk.rewards",
            rewards_schema(),
            RegistryModuleLoad::Always,
        )],
    );
    let lifecycle = load_js_mod(&mut registry, "typed_rewards_contexts_mod", "Typed Rewards Contexts Mod", &root);
    assert_eq!(lifecycle, ModLifecycle::EventDriven);

    let rewards_report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.rewards.event").expect("event key"),
        payload_json: serde_json::json!({
            "count": "A",
            "time": "S",
            "merge": "S+",
            "rank": "S+",
            "berry": 321,
            "crew_points": 180,
            "medals": [{ "item_id": 77, "amount": 2, "is_new": true }]
        })
        .to_string()
        .into(),
    });
    assert_eq!(rewards_report.errors, []);

    let items_report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.rewards.medals").expect("event key"),
        payload_json: serde_json::json!({
            "entries": [{ "item_id": 77, "amount": 2, "is_new": true }]
        })
        .to_string()
        .into(),
    });

    assert_eq!(items_report.errors, []);
    assert_eq!(
        items_report.logs,
        ["js trace mod=typed_rewards_contexts_mod typed rewards medals 77"]
    );
    let _ = fs::remove_dir_all(root);
}
