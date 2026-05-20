use std::{ffi::CString, path::Path};

use crate::runtime::{ffi, manifest::PluginManifest};

pub(super) struct PluginApiState {
    pub(super) game_root_utf8: CString,
    pub(super) plugin_root_utf8: CString,
    pub(super) plugin_mods_root_utf8: CString,
    pub(super) context: ffi::ApiContext,
}

impl PluginApiState {
    pub(super) fn new(game_root: &Path, mods_root: &Path, manifest: &PluginManifest) -> Self {
        Self {
            game_root_utf8: ffi::cstring_lossy(&game_root.to_string_lossy()),
            plugin_root_utf8: ffi::cstring_lossy(&manifest.root.to_string_lossy()),
            plugin_mods_root_utf8: ffi::cstring_lossy(&mods_root.to_string_lossy()),
            context: ffi::ApiContext::new(
                manifest.id.clone(),
                mods_root.to_path_buf(),
                manifest.capabilities_required.clone(),
                manifest.lua_modules.clone(),
            ),
        }
    }
}
