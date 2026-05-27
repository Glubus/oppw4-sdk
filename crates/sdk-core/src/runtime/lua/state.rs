use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::runtime::logs as host_logs;

use super::{
    hot_reload::{mod_fingerprint, ModFingerprint},
    module::RegisteredModule,
    runner::{run_initial_mods, run_mod, ModRunReason},
};
#[cfg(test)]
use super::bridge::LuaBridge;
#[cfg(test)]
use crate::runtime::registry::{
    BridgeId, BridgeModContext, BridgeModSource, LanguageBridge, ModId, SdkRegistry,
};

#[derive(Default)]
pub(super) struct LuaHost {
    mods_root: PathBuf,
    modules: Vec<RegisteredModule>,
    executed: HashSet<String>,
    fingerprints: HashMap<String, ModFingerprint>,
    last_reload_attempts: HashMap<String, Instant>,
    hot_reload_started: bool,
}

impl LuaHost {
    pub(super) fn reset(&mut self, mods_root: &Path) {
        self.mods_root = mods_root.to_path_buf();
        self.modules.clear();
        self.executed.clear();
        self.fingerprints.clear();
        self.last_reload_attempts.clear();
    }

    pub(super) fn mods_root(&self) -> PathBuf {
        self.mods_root.clone()
    }

    pub(super) fn start_hot_reload(&mut self) -> bool {
        if self.hot_reload_started {
            return false;
        }
        self.hot_reload_started = true;
        true
    }

    pub(super) fn register_module(&mut self, entry: RegisteredModule) -> Result<(), String> {
        if let Some(existing) = self.modules.iter().find(|existing| {
            existing
                .module_name
                .eq_ignore_ascii_case(&entry.module_name)
                && !existing.plugin_id.eq_ignore_ascii_case(&entry.plugin_id)
        }) {
            return Err(format!(
                "module {} is already registered by {}",
                entry.module_name, existing.plugin_id
            ));
        }
        self.modules.retain(|existing| {
            !(existing.plugin_id.eq_ignore_ascii_case(&entry.plugin_id)
                && existing
                    .module_name
                    .eq_ignore_ascii_case(&entry.module_name))
        });
        self.modules.push(entry);
        Ok(())
    }

    pub(super) fn run_ready_mods(&mut self) {
        let discovered = lua_api::discover_mods(&self.mods_root);
        self.log_diagnostic(format!(
            "run_ready_mods root={} discovered={} modules={}",
            self.mods_root.display(),
            discovered.len(),
            self.modules
                .iter()
                .map(|module| format!("{}:{}", module.plugin_id, module.module_name))
                .collect::<Vec<_>>()
                .join(",")
        ));
        let mut ready = Vec::new();
        for mod_entry in discovered {
            if self.executed.contains(&mod_entry.manifest.id) {
                continue;
            }
            if !self.mod_dependencies_available(&mod_entry.manifest.uses_plugins) {
                continue;
            }
            self.log_diagnostic(format!(
                "mod queued id={} uses={:?}",
                mod_entry.manifest.id, mod_entry.manifest.uses_plugins
            ));
            ready.push((
                mod_entry.clone(),
                self.modules_for_mod(&mod_entry.manifest.uses_plugins),
            ));
        }

        for (mod_entry, ok) in run_initial_mods(ready) {
            if ok {
                self.executed.insert(mod_entry.manifest.id.clone());
                if let Some(fingerprint) = mod_fingerprint(&mod_entry) {
                    self.fingerprints
                        .insert(mod_entry.manifest.id.clone(), fingerprint);
                }
            } else {
                self.log_diagnostic(format!("mod failed id={}", mod_entry.manifest.id));
            }
        }
    }

    #[cfg(test)]
    pub(super) fn load_ready_mods_into_registry(&self, registry: &mut SdkRegistry) -> Vec<String> {
        let mut bridge = LuaBridge::new(self.modules.clone());
        let bridge_id = BridgeId::new("lua").expect("static bridge id");
        let mut loaded = Vec::new();
        for mod_entry in lua_api::discover_mods(&self.mods_root) {
            if !self.mod_dependencies_available(&mod_entry.manifest.uses_plugins) {
                continue;
            }
            let mod_id = ModId::new(mod_entry.manifest.id.clone()).expect("mod id");
            let report = bridge.load_mod(BridgeModContext {
                mod_id: mod_id.clone(),
                bridge_id: bridge_id.clone(),
                name: mod_entry.manifest.name.clone(),
                source: bridge_source_from_lua(&mod_entry.source),
                entry_file: mod_entry.manifest.entry_lua.clone(),
                uses_plugins: mod_entry.manifest.uses_plugins.clone(),
            });
            if registry
                .register_loaded_mod(mod_id, bridge_id.clone(), report)
                .is_ok()
            {
                loaded.push(mod_entry.manifest.id);
            }
        }
        loaded
    }

    pub(super) fn reload_changed_directory_mods(&mut self) {
        for mod_entry in lua_api::discover_mods(&self.mods_root) {
            if !matches!(mod_entry.source, lua_api::ModSource::Directory(_)) {
                continue;
            }
            if !self.executed.contains(&mod_entry.manifest.id) {
                continue;
            }
            if !self.mod_dependencies_available(&mod_entry.manifest.uses_plugins) {
                continue;
            }
            let Some(fingerprint) = mod_fingerprint(&mod_entry) else {
                continue;
            };
            if self
                .fingerprints
                .get(&mod_entry.manifest.id)
                .is_some_and(|known| *known == fingerprint)
            {
                continue;
            }
            if !self.reload_debounce_elapsed(&mod_entry.manifest.id) {
                continue;
            }
            if self.run_mod(&mod_entry, ModRunReason::HotReload) {
                self.fingerprints
                    .insert(mod_entry.manifest.id.clone(), fingerprint);
            }
        }
    }

    fn reload_debounce_elapsed(&mut self, mod_id: &str) -> bool {
        let now = Instant::now();
        if self
            .last_reload_attempts
            .get(mod_id)
            .is_some_and(|last| now.duration_since(*last) < Duration::from_millis(750))
        {
            return false;
        }
        self.last_reload_attempts.insert(mod_id.to_string(), now);
        true
    }

    fn run_mod(&self, mod_entry: &lua_api::LuaMod, reason: ModRunReason) -> bool {
        run_mod(
            mod_entry,
            self.modules_for_mod(&mod_entry.manifest.uses_plugins),
            reason,
        )
    }

    fn mod_dependencies_available(&self, uses_plugins: &[String]) -> bool {
        uses_plugins.iter().all(|plugin| {
            self.modules
                .iter()
                .any(|module| module.plugin_id.eq_ignore_ascii_case(plugin))
        })
    }

    fn modules_for_mod(&self, uses_plugins: &[String]) -> Vec<RegisteredModule> {
        self.modules
            .iter()
            .filter(|module| {
                is_global_module(module)
                    || uses_plugins
                        .iter()
                        .any(|plugin| module.plugin_id.eq_ignore_ascii_case(plugin))
            })
            .cloned()
            .collect()
    }

    fn log_diagnostic(&self, message: impl AsRef<str>) {
        host_logs::write_mod("_lua_host", message.as_ref());
    }
}

fn is_global_module(module: &RegisteredModule) -> bool {
    module.module_name.starts_with("std.")
}

#[cfg(test)]
fn bridge_source_from_lua(source: &lua_api::ModSource) -> BridgeModSource {
    match source {
        lua_api::ModSource::Directory(path) => BridgeModSource::Directory(path.clone()),
        lua_api::ModSource::Zip { path, root } => BridgeModSource::Zip {
            path: path.clone(),
            root: root.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    unsafe extern "system" fn noop_register(
        _module_context: *mut c_void,
        _lua: *mut c_void,
    ) -> i32 {
        0
    }

    #[test]
    fn duplicate_module_names_from_different_plugins_are_rejected() {
        let mut host = LuaHost::default();
        host.register_module(module("skin_patcher", "shared"))
            .expect("first module");

        let error = host
            .register_module(module("fx_director", "shared"))
            .expect_err("duplicate module");

        assert!(error.contains("already registered"));
    }

    #[test]
    fn same_plugin_can_replace_own_module_registration() {
        let mut host = LuaHost::default();
        host.register_module(module("skin_patcher", "skin_patcher"))
            .expect("first module");
        host.register_module(module("skin_patcher", "skin_patcher"))
            .expect("replacement");

        assert_eq!(host.modules.len(), 1);
    }

    #[test]
    fn standard_modules_are_available_to_plugin_mods_without_extra_dependency() {
        let mut host = LuaHost::default();
        host.register_module(module("sdk_data", "std.character"))
            .expect("std character");
        host.register_module(module("moveset_patcher", "moveset_patcher"))
            .expect("moveset module");

        let modules = host.modules_for_mod(&["moveset_patcher".to_string()]);
        let names = modules
            .iter()
            .map(|module| module.module_name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, ["std.character", "moveset_patcher"]);
    }

    #[test]
    fn registry_loader_infers_boot_once_for_declarative_lua_mod() {
        let root = temp_root("lua-host-registry");
        let mod_root = root.join("ace_moveset");
        std::fs::create_dir_all(&mod_root).expect("mod dir");
        std::fs::write(
            mod_root.join("mod.toml"),
            r#"
            [mod]
            id = "ace_moveset"
            name = "Ace Moveset"

            [entry]
            lua = "mod.lua"
            "#,
        )
        .expect("manifest");
        std::fs::write(
            mod_root.join("mod.lua"),
            r#"
            local character = require("std.character")
            local moveset_patcher = require("moveset_patcher")
            local moveset = moveset_patcher.patch({ payload_file = "ace_moveset.bin" })
            character.find("ace"):replace_movesets(moveset)
            "#,
        )
        .expect("script");
        std::fs::write(mod_root.join("ace_moveset.bin"), [1_u8, 2, 3]).expect("payload");

        let mut host = LuaHost::default();
        host.reset(&root);
        let mut registry = SdkRegistry::new();

        let loaded = host.load_ready_mods_into_registry(&mut registry);

        assert_eq!(loaded, ["ace_moveset"]);
        let boot_mutations = registry.drain_boot_mutations();
        assert_eq!(boot_mutations.len(), 1);
        assert_eq!(boot_mutations[0].source_mod.as_str(), "ace_moveset");
        assert_eq!(boot_mutations[0].key.as_str(), "moveset.replace");
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }

    fn module(plugin_id: &str, module_name: &str) -> RegisteredModule {
        RegisteredModule {
            plugin_id: plugin_id.to_string(),
            module_name: module_name.to_string(),
            context: 0,
            register: noop_register,
            permissions: Default::default(),
        }
    }
}
