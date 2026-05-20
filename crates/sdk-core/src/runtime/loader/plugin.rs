use std::{
    fs,
    path::{Path, PathBuf},
};

use plugin_abi::{PluginInitFn, OPPW4_PLUGIN_INIT_SYMBOL};

use crate::log;

use crate::runtime::{ffi, logs, manifest::PluginManifest, win};

use super::{paths::path_to_wide, remember_loaded_plugin, state::PluginApiState};

pub(super) struct LoadedPlugin {
    _id: String,
    _path: PathBuf,
    _module: usize,
    _api_state: PluginApiState,
}

pub(super) unsafe fn load_plugin(
    game_root: &Path,
    mods_root: &Path,
    manifest: &PluginManifest,
) -> bool {
    prepare_plugin_load(mods_root, manifest);

    let Some(module) = load_plugin_library(manifest) else {
        return false;
    };
    let Some(init) = find_init_symbol(module, manifest) else {
        return false;
    };
    let api_state = PluginApiState::new(game_root, mods_root, manifest);

    if !initialize_plugin(init, game_root, manifest, &api_state) {
        return false;
    }

    remember_loaded_plugin(LoadedPlugin {
        _id: manifest.id.clone(),
        _path: manifest.entry_path.clone(),
        _module: module as usize,
        _api_state: api_state,
    });
    log_initialized(manifest);
    true
}

fn prepare_plugin_load(mods_root: &Path, manifest: &PluginManifest) {
    logs::register(manifest.id.clone(), manifest.log_root.clone());
    let _ = fs::create_dir_all(mods_root);
}

fn load_plugin_library(manifest: &PluginManifest) -> Option<*mut std::ffi::c_void> {
    let wide = path_to_wide(&manifest.entry_path);
    let module = win::load_library(&wide);
    if module.is_null() {
        log::write_line(format!(
            "plugin host: load failed id={} path={}",
            manifest.id,
            manifest.entry_path.display()
        ));
        return None;
    }
    Some(module)
}

unsafe fn find_init_symbol(
    module: *mut std::ffi::c_void,
    manifest: &PluginManifest,
) -> Option<PluginInitFn> {
    let proc = win::get_proc_address(module, OPPW4_PLUGIN_INIT_SYMBOL.as_ptr().cast());
    if proc.is_null() {
        log::write_line(format!(
            "plugin host: init symbol missing id={} path={}",
            manifest.id,
            manifest.entry_path.display()
        ));
        return None;
    }
    Some(std::mem::transmute(proc))
}

unsafe fn initialize_plugin(
    init: PluginInitFn,
    game_root: &Path,
    manifest: &PluginManifest,
    api_state: &PluginApiState,
) -> bool {
    let api = ffi::build_api(
        game_root,
        &api_state.game_root_utf8,
        &api_state.plugin_root_utf8,
        &api_state.mods_root_utf8,
        &api_state.config_root_utf8,
        &api_state.context,
    );
    let result = init(&api);
    if result != 0 {
        log::write_line(format!(
            "plugin host: init failed id={} path={} result={result}",
            manifest.id,
            manifest.entry_path.display()
        ));
        return false;
    }
    true
}

fn log_initialized(manifest: &PluginManifest) {
    log::write_line(format!(
        "plugin host: initialized id={} path={}",
        manifest.id,
        manifest.entry_path.display()
    ));
}
