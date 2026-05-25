use std::{collections::HashSet, fs, path::Path};

use crate::log;

use super::{paths::mods_root, plugin::load_plugin};
use crate::runtime::manifest::PluginManifest;

const SDK_SERVICES_DIR: &str = "sdk";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SdkService {
    id: &'static str,
    dll: &'static str,
    lua_modules: &'static [&'static str],
    requires: &'static [&'static str],
    provides: &'static [&'static str],
}

const CORE_CAPABILITIES: &[&str] = &[
    "plugin.host",
    "config.schema",
    "lua.runtime",
    "lua.module",
    "mod.discovery",
    "files.virtualize",
    "memory.read",
    "memory.scan",
    "memory.write",
    "hooks.install",
    "std.character.extend",
    "signals.subscribe",
    "signals.emit",
];

const SDK_SERVICES: &[SdkService] = &[
    SdkService {
        id: "sdk_runtime",
        dll: "runtime.dll",
        lua_modules: &["sdk.runtime.fx"],
        requires: &[
            "plugin.host",
            "config.schema",
            "lua.module",
            "hooks.install",
            "memory.read",
            "memory.scan",
            "memory.write",
            "std.character.extend",
            "signals.emit",
        ],
        provides: &[
            "game.runtime",
            "game.active_character",
            "game.status",
            "game.mission.difficulty",
            "game.mission.ranks",
            "game.mission.rewards",
            "runtime.fx",
            "std.character.extend",
        ],
    },
    SdkService {
        id: "sdk_debug",
        dll: "debug.dll",
        lua_modules: &[],
        requires: &[
            "plugin.host",
            "config.schema",
            "memory.read",
            "signals.emit",
        ],
        provides: &["debug.memory"],
    },
    SdkService {
        id: "sdk_overlay",
        dll: "overlay.dll",
        lua_modules: &[],
        requires: &["plugin.host", "config.schema", "signals.subscribe"],
        provides: &["ui.overlay"],
    },
    SdkService {
        id: "sdk_linkdata",
        dll: "linkdata.dll",
        lua_modules: &[],
        requires: &["plugin.host", "files.virtualize"],
        provides: &["linkdata.read", "linkdata.patch"],
    },
    SdkService {
        id: "sdk_rdb",
        dll: "rdb.dll",
        lua_modules: &["sdk.rdb.patcher"],
        requires: &[
            "plugin.host",
            "files.virtualize",
            "lua.module",
            "std.character.extend",
        ],
        provides: &["rdb.read", "rdb.patch", "rdb.skin", "std.character.extend"],
    },
];

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
    report.manifests = manifests.len();

    let mut loaded = HashSet::new();
    let mut capabilities = CORE_CAPABILITIES
        .iter()
        .map(|capability| (*capability).to_string())
        .collect::<HashSet<_>>();
    while !manifests.is_empty() {
        let before = manifests.len();
        let mut deferred = Vec::new();
        for manifest in manifests {
            if !dependencies_loaded(&manifest.dependencies, &loaded) {
                deferred.push(manifest);
                continue;
            }
            if !capabilities_available(&manifest.capabilities_required, &capabilities) {
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

fn dependencies_loaded(dependencies: &[String], loaded: &HashSet<String>) -> bool {
    dependencies.iter().all(|dependency| {
        loaded
            .iter()
            .any(|loaded_id| loaded_id.eq_ignore_ascii_case(dependency))
    })
}

fn capabilities_available(required: &[String], available: &HashSet<String>) -> bool {
    required.iter().all(|required_capability| {
        available.iter().any(|available_capability| {
            available_capability.eq_ignore_ascii_case(required_capability)
        })
    })
}

fn log_unresolved_manifests(
    manifests: &[crate::runtime::manifest::PluginManifest],
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
        .filter(|path| path.is_dir() && !is_sdk_services_dir(path))
        .collect::<Vec<_>>();
    dirs.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    Some(dirs)
}

fn is_sdk_services_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(SDK_SERVICES_DIR))
}

fn plugin_manifest(plugin_dir: std::path::PathBuf) -> Option<PluginManifest> {
    PluginManifest::read_from_dir(&plugin_dir)
}

fn sdk_service_manifests(plugin_root: &Path) -> Vec<PluginManifest> {
    let sdk_root = plugin_root.join(SDK_SERVICES_DIR);
    SDK_SERVICES
        .iter()
        .filter_map(|service| sdk_service_manifest(&sdk_root, *service))
        .collect()
}

fn sdk_service_manifest(sdk_root: &Path, service: SdkService) -> Option<PluginManifest> {
    let entry_path = sdk_root.join(service.dll);
    if !entry_path.is_file() {
        log::write_line(format!(
            "plugin host: sdk service missing id={} path={}",
            service.id,
            entry_path.display()
        ));
        return None;
    }
    Some(PluginManifest::sdk_service(
        service.id,
        service.dll,
        sdk_root,
        service.lua_modules,
        service.requires,
        service.provides,
    ))
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
        fs::create_dir_all(root.join("sdk")).expect("sdk");
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

    #[test]
    fn capabilities_are_case_insensitive() {
        let mut available = HashSet::new();
        available.insert("game.runtime".to_string());

        assert!(capabilities_available(
            &["GAME.RUNTIME".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["linkdata.patch".to_string()],
            &available
        ));
    }

    #[test]
    fn core_capabilities_are_available_before_services() {
        let available = CORE_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect::<HashSet<_>>();

        assert!(capabilities_available(
            &["FILES.VIRTUALIZE".to_string()],
            &available
        ));
        assert!(capabilities_available(
            &["lua.module".to_string(), "std.character.extend".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["linkdata.patch".to_string()],
            &available
        ));
    }

    #[test]
    fn sdk_service_manifests_use_sdk_folder_dlls() {
        let root = temp_root("sdk-services");
        let sdk_root = root.join("sdk");
        fs::create_dir_all(&sdk_root).expect("sdk dir");
        fs::write(sdk_root.join("runtime.dll"), []).expect("runtime dll");
        fs::write(sdk_root.join("debug.dll"), []).expect("debug dll");
        fs::write(sdk_root.join("overlay.dll"), []).expect("overlay dll");
        fs::write(sdk_root.join("linkdata.dll"), []).expect("linkdata dll");
        fs::write(sdk_root.join("rdb.dll"), []).expect("rdb dll");

        let manifests = sdk_service_manifests(&root);

        assert_eq!(
            manifests
                .iter()
                .map(|manifest| manifest.id.as_str())
                .collect::<Vec<_>>(),
            [
                "sdk_runtime",
                "sdk_debug",
                "sdk_overlay",
                "sdk_linkdata",
                "sdk_rdb"
            ]
        );
        assert_eq!(manifests[0].entry_path, sdk_root.join("runtime.dll"));
        assert!(manifests[0]
            .capabilities_required
            .iter()
            .any(|capability| capability == "config.schema"));
        assert_eq!(manifests[0].lua_modules, ["sdk.runtime.fx"]);
        assert_eq!(manifests[3].entry_path, sdk_root.join("linkdata.dll"));
        assert_eq!(manifests[4].lua_modules, ["sdk.rdb.patcher"]);
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }
}
