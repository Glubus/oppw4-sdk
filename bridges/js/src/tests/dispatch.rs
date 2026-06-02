use super::support::*;

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
        payload_json: serde_json::json!({ "value": 42 }).to_string().into(),
    });

    assert_eq!(report.errors, []);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn dispatch_uses_handlers_registered_from_relative_imports() {
    let root = temp_root("js-bridge-relative-import-dispatch");
    fs::create_dir_all(root.join("handlers")).expect("temp dir");
    fs::write(root.join("mod.js"), r#"import "./handlers/events";"#).expect("entry script");
    fs::write(
        root.join("handlers/events.js"),
        r#"
        oppw4.events.on("sdk.test.imported", (ctx) => {
            if (ctx.payload.value !== 7) {
                throw new Error("bad payload");
            }
            oppw4.trace("imported handler reached");
        });
        "#,
    )
    .expect("imported script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("imported_callback_mod").expect("mod id"),
            name: "Imported Callback Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::EventDriven);
    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.imported").expect("event key"),
        payload_json: serde_json::json!({ "value": 7 }).to_string().into(),
    });

    assert_eq!(report.errors, []);
    assert_eq!(
        report.logs,
        ["js trace mod=imported_callback_mod imported handler reached"]
    );
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
        payload_json: "{}".to_string().into(),
    });

    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].mod_id.as_str(), "callback_error_mod");
    assert!(report.errors[0].message.contains("js handler call failed"));
    let _ = fs::remove_dir_all(root);
}
#[test]
fn trace_messages_are_returned_as_load_logs() {
    let root = temp_root("js-bridge-trace-logs");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        oppw4.trace("registry probe ok");
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("trace_mod").expect("mod id"),
            name: "Trace Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(
        registry.drain_load_logs(),
        ["js trace mod=trace_mod registry probe ok"]
    );
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
        payload_json: "{}".to_string().into(),
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
        oppw4.events.on("sdk.test.invalid_json", (event) => {
            event.payload;
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    load_js_mod(&mut registry, "invalid_json_mod", "Invalid Json Mod", &root);

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.invalid_json").expect("event key"),
        payload_json: "{".to_string().into(),
    });

    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].mod_id.as_str(), "invalid_json_mod");
    assert!(report.errors[0].message.contains("js handler call failed"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn query_returns_rank_override_from_js_handler() {
    let root = temp_root("js-bridge-query-rank-override");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { rank, snapshot } from "sdk";

        rank.on_calc_count((ctx) => {
            if (ctx.kind !== "count") {
                throw new Error("bad calc kind");
            }
            if (ctx.count !== 1250) {
                throw new Error("bad count payload");
            }
            if (ctx.mission.id !== 35 || ctx.mission.mode !== "free_log") {
                throw new Error("bad calc mission");
            }
            if (ctx.difficulty.key !== "hard") {
                throw new Error("bad calc difficulty");
            }
            if (ctx.player.active_character_ids[0] !== "zoro") {
                throw new Error("bad calc player");
            }
            if (snapshot.mission.id !== 35 || snapshot.difficulty.key !== "hard") {
                throw new Error("bad runtime snapshot");
            }
            return "S+";
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![
            schema_module("sdk", "sdk.rank", rank_schema(), RegistryModuleLoad::Always),
            schema_module_with_invoke(
                "sdk",
                "sdk.snapshot",
                snapshot_schema(),
                RegistryModuleLoad::Always,
                snapshot_invoke,
            ),
        ],
    );
    load_js_mod(&mut registry, "rank_query_mod", "Rank Query", &root);

    let report = registry.query_event(&EventEnvelope {
        key: EventKey::new("sdk.runtime.rank.calc_count").expect("event key"),
        payload_json: serde_json::json!({ "value_u32": 1250 }).to_string().into(),
    });

    assert_eq!(report.errors, []);
    assert_eq!(report.result_json.as_deref(), Some(r#""S+""#));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn snapshot_module_projects_runtime_state_as_properties() {
    let root = temp_root("js-bridge-snapshot-module");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        import { snapshot } from "sdk";

        if (snapshot.mission.id !== 35 || snapshot.mission.mode !== "free_log") {
            throw new Error("bad snapshot mission");
        }
        if (snapshot.difficulty.key !== "hard") {
            throw new Error("bad snapshot difficulty");
        }
        if (snapshot.player.active_character_ids[0] !== "zoro") {
            throw new Error("bad snapshot player");
        }
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(
        &mut registry,
        vec![schema_module_with_invoke(
            "sdk",
            "sdk.snapshot",
            snapshot_schema(),
            RegistryModuleLoad::Always,
            snapshot_invoke,
        )],
    );

    let lifecycle = registry
        .load_supported_mod(BridgeLoadRequest {
            mod_id: ModId::new("snapshot_mod").expect("mod id"),
            name: "Snapshot Mod".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.js".to_string(),
            uses_plugins: Vec::new(),
        })
        .expect("load mod");

    assert_eq!(lifecycle, ModLifecycle::BootOnce);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn invalid_json_payload_is_lazy_when_payload_is_not_read() {
    let root = temp_root("js-bridge-invalid-json-lazy");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        oppw4.events.on("sdk.test.invalid_json_lazy", (event) => {
            if (event.eventKey !== "sdk.test.invalid_json_lazy") {
                throw new Error("bad event key");
            }
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    load_js_mod(
        &mut registry,
        "invalid_json_lazy_mod",
        "Invalid Json Lazy Mod",
        &root,
    );

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.invalid_json_lazy").expect("event key"),
        payload_json: "{".to_string().into(),
    });

    assert_eq!(report.errors, []);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn payload_is_parsed_once_for_multiple_handlers_in_same_vm() {
    let root = temp_root("js-bridge-lazy-parse-once");
    fs::create_dir_all(&root).expect("temp dir");
    fs::write(
        root.join("mod.js"),
        r#"
        const parse = JSON.parse;
        globalThis.parseCount = 0;
        JSON.parse = (json) => {
            globalThis.parseCount += 1;
            return parse(json);
        };
        oppw4.events.on("sdk.test.lazy_parse", (event) => {
            if (event.payload.value !== 42) {
                throw new Error("bad first payload");
            }
        });
        oppw4.events.on("sdk.test.lazy_parse", (event) => {
            if (event.payload.value !== 42) {
                throw new Error("bad second payload");
            }
            oppw4.trace("parse_count=" + globalThis.parseCount);
        });
        "#,
    )
    .expect("script");

    let mut registry = BridgeRegistry::new();
    register_js_bridge(&mut registry, Vec::new());
    load_js_mod(&mut registry, "lazy_parse_mod", "Lazy Parse Mod", &root);

    let report = registry.dispatch_event(&EventEnvelope {
        key: EventKey::new("sdk.test.lazy_parse").expect("event key"),
        payload_json: serde_json::json!({ "value": 42 }).to_string().into(),
    });

    assert_eq!(report.errors, []);
    assert_eq!(report.metrics.handler_count, 2);
    assert_eq!(report.metrics.vm_batch_count, 1);
    assert_eq!(report.logs, ["js trace mod=lazy_parse_mod parse_count=1"]);
    let _ = fs::remove_dir_all(root);
}
