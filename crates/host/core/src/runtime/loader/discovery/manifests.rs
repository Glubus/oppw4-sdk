use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{log, runtime::manifest::PluginManifest};

use super::services::SDK_SERVICES_DIR;

pub(super) fn plugin_dirs(plugin_root: &Path) -> Option<Vec<PathBuf>> {
    let entries = match fs::read_dir(plugin_root) {
        Ok(entries) => entries,
        Err(_) => {
            log::write_line(format!(
                "plugin host: plugin dir not readable path={}",
                plugin_root.display()
            ));
            return None;
        }
    };

    let mut dirs = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && !is_sdk_services_dir(path))
        .collect::<Vec<_>>();
    dirs.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    Some(dirs)
}

pub(super) fn plugin_manifest(plugin_dir: PathBuf) -> Option<PluginManifest> {
    PluginManifest::read_from_dir(&plugin_dir)
}

pub(super) fn reject_duplicate_manifests(manifests: Vec<PluginManifest>) -> Vec<PluginManifest> {
    let mut seen = HashSet::new();
    let mut unique = Vec::with_capacity(manifests.len());
    for manifest in manifests {
        let key = manifest.id.to_ascii_lowercase();
        if seen.insert(key) {
            unique.push(manifest);
        } else {
            log::write_line(format!(
                "plugin host: duplicate plugin id rejected id={} path={}",
                manifest.id,
                manifest.entry_path.display()
            ));
        }
    }
    unique
}

pub(super) fn log_unresolved_manifests(
    manifests: &[PluginManifest],
    loaded: &HashSet<String>,
    capabilities: &HashSet<String>,
) {
    for manifest in manifests {
        let missing_dependencies = manifest
            .dependencies
            .iter()
            .filter(|dependency| {
                !loaded
                    .iter()
                    .any(|loaded_id| loaded_id.eq_ignore_ascii_case(dependency))
            })
            .cloned()
            .collect::<Vec<_>>();
        let missing_capabilities = manifest
            .capabilities_required
            .iter()
            .filter(|capability| {
                !capabilities
                    .iter()
                    .any(|loaded_capability| loaded_capability.eq_ignore_ascii_case(capability))
            })
            .cloned()
            .collect::<Vec<_>>();
        log::write_line(format!(
            "plugin host: manifest unresolved id={} missing_dependencies={missing_dependencies:?} missing_capabilities={missing_capabilities:?}",
            manifest.id
        ));
    }
}

fn is_sdk_services_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(SDK_SERVICES_DIR))
}
