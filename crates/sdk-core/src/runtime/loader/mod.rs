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

use super::logs;
use plugin::LoadedPlugin;

static LOADED: OnceLock<Mutex<Vec<LoadedPlugin>>> = OnceLock::new();

pub fn initialize(game_root: &Path, plugin_root: &Path, session_stamp: Option<String>) {
    prepare_runtime(game_root, plugin_root, session_stamp);
    let report = discovery::load_plugins(game_root, plugin_root);
    log::write_line(format!(
        "plugin host: scanned={} manifests={} loaded={}",
        report.scanned, report.manifests, report.loaded
    ));
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
}

fn remember_loaded_plugin(plugin: LoadedPlugin) {
    if let Some(plugins) = LOADED.get() {
        plugins.lock().expect("plugin list lock").push(plugin);
    }
}
