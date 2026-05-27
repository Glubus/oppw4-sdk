use std::{
    ffi::CString,
    path::{Path, PathBuf},
};

use crate::runtime::{ffi, manifest::PluginManifest};
use plugin_sdk::manifest::sanitize_plugin_id;

pub(super) struct PluginApiState {
    pub(super) game_root_utf8: CString,
    pub(super) plugin_root_utf8: CString,
    pub(super) mods_root_utf8: CString,
    pub(super) config_root_utf8: CString,
    pub(super) context: ffi::ApiContext,
}

impl PluginApiState {
    pub(super) fn new(game_root: &Path, mods_root: &Path, manifest: &PluginManifest) -> Self {
        let mut capabilities = manifest.capabilities_required.clone();
        capabilities.extend(manifest.capabilities_provided.iter().cloned());
        Self {
            game_root_utf8: ffi::cstring_lossy(&game_root.to_string_lossy()),
            plugin_root_utf8: ffi::cstring_lossy(&manifest.root.to_string_lossy()),
            mods_root_utf8: ffi::cstring_lossy(&mods_root.to_string_lossy()),
            config_root_utf8: ffi::cstring_lossy(&plugin_config_root(manifest).to_string_lossy()),
            context: ffi::ApiContext::new(
                manifest.id.clone(),
                mods_root.to_path_buf(),
                capabilities,
            ),
        }
    }
}

fn plugin_config_root(manifest: &PluginManifest) -> PathBuf {
    manifest
        .root
        .parent()
        .unwrap_or(&manifest.root)
        .join("configs")
        .join(sanitize_plugin_id(&manifest.id))
}
