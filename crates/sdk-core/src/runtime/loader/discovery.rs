use std::{collections::HashSet, fs, path::Path};

use plugin_sdk::{
    CAP_CONFIG_SCHEMA, CAP_FILES_VIRTUALIZE, CAP_HOOKS_INSTALL, CAP_LUA_MODULE, CAP_LUA_RUNTIME,
    CAP_MEMORY_READ, CAP_MEMORY_SCAN, CAP_MEMORY_WRITE, CAP_MOD_DISCOVERY, CAP_PLUGIN_HOST,
    CAP_SIGNALS_EMIT, CAP_SIGNALS_SUBSCRIBE, CAP_STD_CHARACTER_EXTEND,
};

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

const PUBLIC_CORE_CAPABILITIES: &[&str] = &[
    CAP_PLUGIN_HOST,
    CAP_CONFIG_SCHEMA,
    CAP_LUA_RUNTIME,
    CAP_LUA_MODULE,
    CAP_MOD_DISCOVERY,
    CAP_MEMORY_READ,
    CAP_MEMORY_SCAN,
    CAP_SIGNALS_SUBSCRIBE,
];

const SDK_INTERNAL_CORE_CAPABILITIES: &[&str] = &[
    CAP_PLUGIN_HOST,
    CAP_CONFIG_SCHEMA,
    CAP_LUA_RUNTIME,
    CAP_LUA_MODULE,
    CAP_MOD_DISCOVERY,
    CAP_FILES_VIRTUALIZE,
    CAP_HOOKS_INSTALL,
    CAP_MEMORY_READ,
    CAP_MEMORY_SCAN,
    CAP_MEMORY_WRITE,
    CAP_SIGNALS_EMIT,
    CAP_SIGNALS_SUBSCRIBE,
    CAP_STD_CHARACTER_EXTEND,
];

const SDK_SERVICES: &[SdkService] = &[
    SdkService {
        id: "sdk_runtime",
        dll: "runtime.dll",
        lua_modules: &[
            "sdk.runtime.fx",
            "sdk.runtime.player",
            "sdk.runtime.ranks",
            "sdk.runtime.difficulty",
        ],
        requires: &[
            CAP_PLUGIN_HOST,
            CAP_CONFIG_SCHEMA,
            CAP_LUA_MODULE,
            CAP_HOOKS_INSTALL,
            CAP_MEMORY_READ,
            CAP_MEMORY_SCAN,
            CAP_MEMORY_WRITE,
            CAP_STD_CHARACTER_EXTEND,
            CAP_SIGNALS_SUBSCRIBE,
            CAP_SIGNALS_EMIT,
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
            CAP_PLUGIN_HOST,
            CAP_CONFIG_SCHEMA,
            CAP_MEMORY_READ,
            CAP_SIGNALS_EMIT,
        ],
        provides: &["debug.memory"],
    },
    SdkService {
        id: "sdk_overlay",
        dll: "overlay.dll",
        lua_modules: &[],
        requires: &[CAP_PLUGIN_HOST, CAP_CONFIG_SCHEMA, CAP_SIGNALS_SUBSCRIBE],
        provides: &["ui.overlay"],
    },
    SdkService {
        id: "sdk_linkdata",
        dll: "linkdata.dll",
        lua_modules: &[],
        requires: &[CAP_PLUGIN_HOST, CAP_FILES_VIRTUALIZE],
        provides: &["linkdata.read", "linkdata.patch"],
    },
    SdkService {
        id: "sdk_rdb",
        dll: "rdb.dll",
        lua_modules: &["sdk.rdb.patcher"],
        requires: &[
            CAP_PLUGIN_HOST,
            CAP_FILES_VIRTUALIZE,
            CAP_LUA_MODULE,
            CAP_STD_CHARACTER_EXTEND,
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
    manifests = reject_duplicate_manifests(manifests);
    report.manifests = manifests.len();

    let mut loaded = HashSet::new();
    let mut capabilities = PUBLIC_CORE_CAPABILITIES
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

fn capabilities_available_for_manifest(
    manifest: &PluginManifest,
    available: &HashSet<String>,
) -> bool {
    if is_sdk_service_id(&manifest.id) {
        return manifest.capabilities_required.iter().all(|capability| {
            has_capability(capability, available)
                || SDK_INTERNAL_CORE_CAPABILITIES
                    .iter()
                    .any(|internal| internal.eq_ignore_ascii_case(capability))
        });
    }
    capabilities_available(&manifest.capabilities_required, available)
}

fn is_sdk_service_id(id: &str) -> bool {
    SDK_SERVICES
        .iter()
        .any(|service| service.id.eq_ignore_ascii_case(id))
}

fn has_capability(required: &str, available: &HashSet<String>) -> bool {
    available
        .iter()
        .any(|capability| capability.eq_ignore_ascii_case(required))
}

fn reject_duplicate_manifests(manifests: Vec<PluginManifest>) -> Vec<PluginManifest> {
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
    fn public_core_capabilities_are_available_before_services() {
        let available = PUBLIC_CORE_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect::<HashSet<_>>();

        assert!(capabilities_available(
            &["lua.module".to_string(), "memory.read".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["linkdata.patch".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["files.virtualize".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["hooks.install".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["memory.write".to_string()],
            &available
        ));
        assert!(!capabilities_available(
            &["signals.emit".to_string()],
            &available
        ));
    }

    #[test]
    fn sdk_services_can_use_internal_core_capabilities() {
        let available = PUBLIC_CORE_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect::<HashSet<_>>();
        let mut service = manifest_for_test("sdk_runtime", "sdk/runtime".into());
        service.capabilities_required = vec![
            "hooks.install".to_string(),
            "memory.write".to_string(),
            "signals.emit".to_string(),
        ];
        let mut plugin = manifest_for_test("third_party", "mods/third_party".into());
        plugin.capabilities_required = service.capabilities_required.clone();

        assert!(capabilities_available_for_manifest(&service, &available));
        assert!(!capabilities_available_for_manifest(&plugin, &available));
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
        assert_eq!(
            manifests[0].lua_modules,
            [
                "sdk.runtime.fx",
                "sdk.runtime.player",
                "sdk.runtime.ranks",
                "sdk.runtime.difficulty"
            ]
        );
        assert_eq!(manifests[3].entry_path, sdk_root.join("linkdata.dll"));
        assert_eq!(manifests[4].lua_modules, ["sdk.rdb.patcher"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_plugin_ids_are_rejected() {
        let root = temp_root("duplicate-plugin-ids");
        let first = manifest_for_test("zoro_elbaf", root.join("a"));
        let duplicate = manifest_for_test("ZORO_ELBAF", root.join("b"));
        let other = manifest_for_test("zoro_elbaf_battle", root.join("c"));

        let manifests = reject_duplicate_manifests(vec![first, duplicate, other]);

        assert_eq!(
            manifests
                .iter()
                .map(|manifest| manifest.id.as_str())
                .collect::<Vec<_>>(),
            ["zoro_elbaf", "zoro_elbaf_battle"]
        );
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }

    fn manifest_for_test(id: &str, root: PathBuf) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            version: "0.1.0".to_string(),
            dependencies: Vec::new(),
            lua_modules: Vec::new(),
            capabilities_required: Vec::new(),
            capabilities_provided: Vec::new(),
            mods_root: root.join("mods"),
            entry_path: root.join(format!("{id}.dll")),
            log_root: root.join("logs"),
            root,
        }
    }
}
