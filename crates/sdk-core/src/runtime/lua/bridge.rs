#![allow(dead_code)]

use mlua::Lua;

use crate::runtime::{
    logs as host_logs,
    registry::{
        BridgeDispatchReport, BridgeId, BridgeLoadReport, BridgeModContext, BridgeModSource,
        EventEnvelope, HandlerDescriptor, LanguageBridge, ModId, MutationEnvelope, MutationKey,
        RegistryError,
    },
};

use super::{
    module::{register_plugin_module, RegisteredModule},
    owned_modules,
};

pub(super) struct LuaBridge {
    modules: Vec<RegisteredModule>,
}

impl LuaBridge {
    pub(super) fn new(modules: Vec<RegisteredModule>) -> Self {
        Self { modules }
    }
}

impl LanguageBridge for LuaBridge {
    fn id(&self) -> BridgeId {
        BridgeId::new("lua").expect("static bridge id")
    }

    fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport {
        let mod_entry = lua_mod_from_context(&context);
        let result = lua_api::run_lua_mod(&mod_entry, |lua| {
            install_bridge_modules(lua, &context.mod_id, &self.modules)
        });
        match result {
            Ok(report) => BridgeLoadReport {
                boot_mutations: report
                    .mutations
                    .into_iter()
                    .filter_map(|mutation| lua_mutation_to_envelope(&context.mod_id, mutation))
                    .collect(),
                logs: report
                    .logs
                    .into_iter()
                    .map(|entry| format!("{}:{}", entry.level, entry.message))
                    .collect(),
                ..BridgeLoadReport::default()
            },
            Err(error) => BridgeLoadReport {
                errors: vec![RegistryError::BridgeLoadError {
                    mod_id: context.mod_id.as_str().to_string(),
                    bridge_id: context.bridge_id.as_str().to_string(),
                    message: format!("{error:?}"),
                }],
                ..BridgeLoadReport::default()
            },
        }
    }

    fn dispatch(
        &mut self,
        handler: &HandlerDescriptor,
        _event: &EventEnvelope,
    ) -> BridgeDispatchReport {
        BridgeDispatchReport {
            errors: vec![crate::runtime::registry::RegistryDispatchError {
                mod_id: handler.mod_id.clone(),
                bridge_id: handler.bridge_id.clone(),
                message: "lua runtime handlers are not wired to sdk registry yet".to_string(),
            }],
            ..BridgeDispatchReport::default()
        }
    }

    fn unload_mod(&mut self, _mod_id: &ModId) {}
}

fn install_bridge_modules(lua: &Lua, mod_id: &ModId, modules: &[RegisteredModule]) -> mlua::Result<()> {
    owned_modules::install(lua)?;
    let mod_id = mod_id.as_str().to_string();
    lua.globals().set(
        "__oppw4_trace",
        lua.create_function(move |_, message: String| {
            host_logs::write_mod("lua_bridge", &format!("trace mod={mod_id} {message}"));
            Ok(())
        })?,
    )?;
    for module in modules {
        if is_sdk_owned_module(module) {
            continue;
        }
        register_plugin_module(lua, module)?;
    }
    Ok(())
}

fn lua_mod_from_context(context: &BridgeModContext) -> lua_api::LuaMod {
    lua_api::LuaMod {
        manifest: lua_api::LuaModManifest {
            id: context.mod_id.as_str().to_string(),
            name: context.name.clone(),
            uses_plugins: context.uses_plugins.clone(),
            entry_lua: context.entry_file.clone(),
        },
        source: match &context.source {
            BridgeModSource::Directory(path) => lua_api::ModSource::Directory(path.clone()),
            BridgeModSource::Zip { path, root } => lua_api::ModSource::Zip {
                path: path.clone(),
                root: root.clone(),
            },
        },
    }
}

fn lua_mutation_to_envelope(mod_id: &ModId, mutation: lua_api::LuaMutation) -> Option<MutationEnvelope> {
    let key = MutationKey::new(mutation.kind.as_str()).ok()?;
    Some(MutationEnvelope {
        key,
        source_mod: mod_id.clone(),
        payload_json: lua_mutation_payload_json(mutation),
    })
}

fn lua_mutation_payload_json(mutation: lua_api::LuaMutation) -> String {
    let character = mutation.character.unwrap_or_default();
    let payload_file = mutation.payload_file.unwrap_or_default();
    let payload_len = mutation.payload.as_ref().map(Vec::len).unwrap_or(0);
    format!(
        r#"{{"mod_id":"{}","character":"{}","entry":{},"payload_file":"{}","payload_len":{}}}"#,
        escape_json_string(&mutation.mod_id),
        escape_json_string(&character),
        mutation.entry.unwrap_or_default(),
        escape_json_string(&payload_file),
        payload_len
    )
}

fn escape_json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_sdk_owned_module(module: &RegisteredModule) -> bool {
    matches!(
        module.module_name.as_str(),
        "std.character" | "character" | "moveset_patcher"
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::runtime::registry::ModLifecycle;

    #[test]
    fn lua_bridge_loads_boot_once_moveset_mutation() {
        let root = temp_root("lua-bridge-moveset");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("ace_moveset.bin"), [1_u8, 2, 3, 4]).expect("payload");
        fs::write(
            root.join("mod.lua"),
            r#"
            local character = require("std.character")
            local moveset_patcher = require("moveset_patcher")
            local moveset = moveset_patcher.patch({ payload_file = "ace_moveset.bin" })
            character.find("ace"):replace_movesets(moveset)
            "#,
        )
        .expect("script");

        let mod_id = ModId::new("ace_moveset").expect("mod id");
        let bridge_id = BridgeId::new("lua").expect("bridge id");
        let mut bridge = LuaBridge::new(Vec::new());
        let report = bridge.load_mod(BridgeModContext {
            mod_id: mod_id.clone(),
            bridge_id: bridge_id.clone(),
            name: "Ace Moveset".to_string(),
            source: BridgeModSource::Directory(root.clone()),
            entry_file: "mod.lua".to_string(),
            uses_plugins: Vec::new(),
        });

        assert_eq!(report.errors, []);
        assert_eq!(report.handlers, []);
        assert_eq!(report.boot_mutations.len(), 1);
        assert_eq!(report.boot_mutations[0].key, MutationKey::new("moveset.replace").unwrap());
        assert!(report.boot_mutations[0].payload_json.contains("ace_moveset.bin"));

        let mut registry = crate::runtime::registry::SdkRegistry::new();
        let lifecycle = registry
            .register_loaded_mod(mod_id, bridge_id, report)
            .expect("registry load");
        assert_eq!(lifecycle, ModLifecycle::BootOnce);

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }
}
