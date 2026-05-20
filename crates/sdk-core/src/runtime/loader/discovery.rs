use std::{collections::HashSet, fs, path::Path};

use crate::log;

use super::{paths::mods_root, plugin::load_plugin};
use crate::runtime::manifest::PluginManifest;

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
    let mut manifests = entries
        .into_iter()
        .filter_map(plugin_manifest)
        .collect::<Vec<_>>();
    report.manifests = manifests.len();

    let mut loaded = HashSet::new();
    while !manifests.is_empty() {
        let before = manifests.len();
        let mut deferred = Vec::new();
        for manifest in manifests {
            if !dependencies_loaded(&manifest.dependencies, &loaded) {
                deferred.push(manifest);
                continue;
            }
            if unsafe { load_plugin(game_root, &mods_root(game_root), &manifest) } {
                loaded.insert(manifest.id.clone());
                report.loaded += 1;
            }
        }
        if deferred.len() == before {
            log_unresolved_dependencies(&deferred, &loaded);
            break;
        }
        manifests = deferred;
    }

    report
}

fn dependencies_loaded(dependencies: &[String], loaded: &HashSet<String>) -> bool {
    dependencies.iter().all(|dependency| {
        loaded
            .iter()
            .any(|loaded_id| loaded_id.eq_ignore_ascii_case(dependency))
    })
}

fn log_unresolved_dependencies(
    manifests: &[crate::runtime::manifest::PluginManifest],
    loaded: &HashSet<String>,
) {
    for manifest in manifests {
        let missing = manifest
            .dependencies
            .iter()
            .filter(|dependency| {
                !loaded
                    .iter()
                    .any(|loaded_id| loaded_id.eq_ignore_ascii_case(dependency))
            })
            .cloned()
            .collect::<Vec<_>>();
        log::write_line(format!(
            "plugin host: dependencies unresolved id={} missing={missing:?}",
            manifest.id
        ));
    }
}

fn plugin_dirs(plugin_root: &Path) -> Option<Vec<std::path::PathBuf>> {
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
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    Some(dirs)
}

fn plugin_manifest(plugin_dir: std::path::PathBuf) -> Option<PluginManifest> {
    PluginManifest::read_from_dir(&plugin_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    #[test]
    fn plugin_dirs_are_sorted_for_stable_load_order() {
        let root = temp_root("plugin-dir-order");
        fs::create_dir_all(root.join("z_plugin")).expect("z");
        fs::create_dir_all(root.join("a_plugin")).expect("a");
        fs::write(root.join("loose.dll"), []).expect("file");

        let dirs = plugin_dirs(&root).expect("dirs");

        assert_eq!(dirs, vec![root.join("a_plugin"), root.join("z_plugin")]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dependencies_are_case_insensitive() {
        let mut loaded = HashSet::new();
        loaded.insert("skin_patcher".to_string());

        assert!(dependencies_loaded(&["SKIN_PATCHER".to_string()], &loaded));
        assert!(!dependencies_loaded(&["fx_director".to_string()], &loaded));
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }
}
