mod discovery;
mod paths;
mod plugin;
mod state;

use std::{
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

use crate::log;
use sdk_bridge::{BridgeRegistry, RegistryModuleDescriptor};

use super::logs;
use plugin::LoadedPlugin;

static LOADED: OnceLock<Mutex<Vec<LoadedPlugin>>> = OnceLock::new();
static BRIDGES: OnceLock<Mutex<BridgeRegistry>> = OnceLock::new();

pub fn initialize(game_root: &Path, plugin_root: &Path, session_stamp: Option<String>) {
    initialize_with_bridge_setup(game_root, plugin_root, session_stamp, |_| {});
}

pub fn initialize_with_bridge_setup(
    game_root: &Path,
    plugin_root: &Path,
    session_stamp: Option<String>,
    setup: impl FnOnce(&mut BridgeRegistry),
) {
    prepare_runtime(game_root, plugin_root, session_stamp);
    if let Some(registry) = BRIDGES.get() {
        setup(&mut registry.lock().expect("bridge registry lock"));
    }
    let report = discovery::load_plugins(game_root, plugin_root);
    log::write_line(format!(
        "plugin host: scanned={} manifests={} loaded={}",
        report.scanned, report.manifests, report.loaded
    ));
    load_mods(game_root);
}

fn prepare_runtime(game_root: &Path, plugin_root: &Path, session_stamp: Option<String>) {
    let _ = fs::create_dir_all(plugin_root);
    logs::initialize(
        session_stamp,
        paths::mods_root(game_root)
            .join("_oppw4")
            .join("logs")
            .join("mods"),
    );
    let _ = LOADED.set(Mutex::new(Vec::new()));
    let _ = BRIDGES.set(Mutex::new(BridgeRegistry::new()));
}

fn remember_loaded_plugin(plugin: LoadedPlugin) {
    if let Some(plugins) = LOADED.get() {
        plugins.lock().expect("plugin list lock").push(plugin);
    }
}

fn load_mods(game_root: &Path) {
    let mods_root = paths::mods_root(game_root);
    let mods = sdk_bridge::discover_mods(&mods_root);
    if mods.is_empty() {
        logs::write_mod(
            "plugin_host",
            &format!("mods scanned=0 root={}", mods_root.display()),
        );
        return;
    }
    let Some(registry) = BRIDGES.get() else {
        log::write_line("plugin host: bridge registry is not initialized");
        return;
    };
    let mut registry = registry.lock().expect("bridge registry lock");
    let total = mods.len();
    let mut loaded = 0usize;
    for mod_entry in mods {
        let mod_id = mod_entry.manifest.id.as_str().to_string();
        logs::write_mod(
            "plugin_host",
            &format!(
                "mod discovered id={} entry={} uses={:?}",
                mod_id, mod_entry.manifest.entry_file, mod_entry.manifest.uses_plugins
            ),
        );
        match registry.load_supported_mod(mod_entry.into_load_request()) {
            Ok(lifecycle) => {
                loaded += 1;
                for line in registry.drain_load_logs() {
                    log::write_line(format!("plugin host: mod log id={mod_id} {line}"));
                    logs::write_mod(&mod_id, &line);
                }
                let line = format!("mod loaded id={mod_id} lifecycle={lifecycle:?}");
                log::write_line(format!("plugin host: {line}"));
                logs::write_mod("plugin_host", &line);
            }
            Err(error) => {
                for line in registry.drain_load_logs() {
                    log::write_line(format!("plugin host: mod log id={mod_id} {line}"));
                    logs::write_mod(&mod_id, &line);
                }
                let line = format!("mod load failed id={mod_id} error={error:?}");
                log::write_line(format!("plugin host: {line}"));
                logs::write_mod("plugin_host", &line);
            }
        }
    }
    log::write_line(format!("plugin host: mods scanned={total} loaded={loaded}"));
    logs::write_mod(
        "plugin_host",
        &format!("mods scanned={total} loaded={loaded}"),
    );
}

pub(crate) fn register_registry_module(module: RegistryModuleDescriptor) -> i32 {
    let Some(registry) = BRIDGES.get() else {
        return -30;
    };
    registry
        .lock()
        .expect("bridge registry lock")
        .register_module(module);
    0
}
