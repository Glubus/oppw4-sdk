use std::{collections::HashSet, path::Path};

use super::{paths::mods_root, plugin::load_plugin};

mod manifests;
mod rules;
mod services;

#[cfg(test)]
mod tests;

use manifests::{
    log_unresolved_manifests, plugin_dirs, plugin_manifest, reject_duplicate_manifests,
};
use rules::dependencies_loaded;
use services::{
    capabilities_available_for_manifest, public_core_capabilities, sdk_service_manifests,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PluginLoadReport {
    pub(super) scanned: usize,
    pub(super) manifests: usize,
    pub(super) loaded: usize,
}

pub(super) fn load_plugins(game_root: &Path, plugin_root: &Path) -> PluginLoadReport {
    let Some(entries) = plugin_dirs(plugin_root) else {
        return PluginLoadReport::default();
    };

    let mut report = PluginLoadReport {
        scanned: entries.len(),
        ..PluginLoadReport::default()
    };
    let mut manifests = sdk_service_manifests(plugin_root);
    manifests.extend(
        entries
            .into_iter()
            .filter_map(plugin_manifest)
            .collect::<Vec<_>>(),
    );
    manifests = reject_duplicate_manifests(manifests);
    report.manifests = manifests.len();

    let mut loaded = HashSet::new();
    let mut capabilities = public_core_capabilities();
    while !manifests.is_empty() {
        let before = manifests.len();
        let mut deferred = Vec::new();
        for manifest in manifests {
            if !dependencies_loaded(&manifest.dependencies, &loaded) {
                deferred.push(manifest);
                continue;
            }
            if !capabilities_available_for_manifest(&manifest, &capabilities) {
                deferred.push(manifest);
                continue;
            }
            if unsafe { load_plugin(game_root, &mods_root(game_root), &manifest) } {
                loaded.insert(manifest.id.clone());
                capabilities.extend(manifest.capabilities_provided.iter().cloned());
                report.loaded += 1;
            }
        }
        if deferred.len() == before {
            log_unresolved_manifests(&deferred, &loaded, &capabilities);
            break;
        }
        manifests = deferred;
    }

    report
}
