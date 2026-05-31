use std::{collections::HashSet, path::Path};

use plugin_sdk::{
    CAP_CONFIG_SCHEMA, CAP_FILES_VIRTUALIZE, CAP_HOOKS_INSTALL, CAP_MEMORY_READ, CAP_MEMORY_SCAN,
    CAP_MEMORY_WRITE, CAP_MOD_DISCOVERY, CAP_PLUGIN_HOST, CAP_REGISTRY_MODULE, CAP_SIGNALS_EMIT,
    CAP_SIGNALS_SUBSCRIBE,
};

use crate::{log, runtime::manifest::PluginManifest};

use super::rules::{capabilities_available, has_capability};

pub(super) const SDK_SERVICES_DIR: &str = "sdk";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SdkService {
    id: &'static str,
    dll: &'static str,
    requires: &'static [&'static str],
    provides: &'static [&'static str],
    registry_modules: &'static [&'static str],
}

const PUBLIC_CORE_CAPABILITIES: &[&str] = &[
    CAP_PLUGIN_HOST,
    CAP_CONFIG_SCHEMA,
    CAP_MOD_DISCOVERY,
    CAP_FILES_VIRTUALIZE,
    CAP_MEMORY_READ,
    CAP_MEMORY_SCAN,
    CAP_REGISTRY_MODULE,
    CAP_SIGNALS_SUBSCRIBE,
];

const SDK_INTERNAL_CORE_CAPABILITIES: &[&str] = &[
    CAP_PLUGIN_HOST,
    CAP_CONFIG_SCHEMA,
    CAP_MOD_DISCOVERY,
    CAP_FILES_VIRTUALIZE,
    CAP_HOOKS_INSTALL,
    CAP_MEMORY_READ,
    CAP_MEMORY_SCAN,
    CAP_MEMORY_WRITE,
    CAP_REGISTRY_MODULE,
    CAP_SIGNALS_EMIT,
    CAP_SIGNALS_SUBSCRIBE,
];

const SDK_SERVICES: &[SdkService] = &[
    SdkService {
        id: "sdk_data",
        dll: "data.dll",
        requires: &[CAP_PLUGIN_HOST, CAP_REGISTRY_MODULE],
        provides: &["game.data", "game.characters", "game.missions"],
        registry_modules: &["sdk.character"],
    },
    SdkService {
        id: "sdk_runtime",
        dll: "runtime.dll",
        requires: &[
            CAP_PLUGIN_HOST,
            "game.data",
            CAP_CONFIG_SCHEMA,
            CAP_HOOKS_INSTALL,
            CAP_MEMORY_READ,
            CAP_MEMORY_SCAN,
            CAP_MEMORY_WRITE,
            CAP_REGISTRY_MODULE,
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
        ],
        registry_modules: &["sdk.player", "sdk.difficulty", "sdk.rank", "sdk.rewards"],
    },
    SdkService {
        id: "sdk_debug",
        dll: "debug.dll",
        requires: &[
            CAP_PLUGIN_HOST,
            CAP_CONFIG_SCHEMA,
            CAP_MEMORY_READ,
            CAP_SIGNALS_EMIT,
        ],
        provides: &["debug.memory"],
        registry_modules: &[],
    },
    SdkService {
        id: "sdk_overlay",
        dll: "overlay.dll",
        requires: &[CAP_PLUGIN_HOST, CAP_CONFIG_SCHEMA, CAP_SIGNALS_SUBSCRIBE],
        provides: &["ui.overlay"],
        registry_modules: &[],
    },
    SdkService {
        id: "sdk_linkdata",
        dll: "linkdata.dll",
        requires: &[CAP_PLUGIN_HOST, CAP_FILES_VIRTUALIZE],
        provides: &["linkdata.read", "linkdata.patch"],
        registry_modules: &[],
    },
    SdkService {
        id: "sdk_rdb",
        dll: "rdb.dll",
        requires: &[CAP_PLUGIN_HOST, CAP_FILES_VIRTUALIZE],
        provides: &["rdb.read", "rdb.patch", "rdb.skin"],
        registry_modules: &[],
    },
];

pub(super) fn public_core_capabilities() -> HashSet<String> {
    PUBLIC_CORE_CAPABILITIES
        .iter()
        .map(|capability| (*capability).to_string())
        .collect()
}

pub(super) fn capabilities_available_for_manifest(
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

pub(super) fn sdk_service_manifests(plugin_root: &Path) -> Vec<PluginManifest> {
    let sdk_root = plugin_root.join(SDK_SERVICES_DIR);
    SDK_SERVICES
        .iter()
        .filter_map(|service| sdk_service_manifest(&sdk_root, *service))
        .collect()
}

fn is_sdk_service_id(id: &str) -> bool {
    SDK_SERVICES
        .iter()
        .any(|service| service.id.eq_ignore_ascii_case(id))
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
        service.requires,
        service.provides,
        service.registry_modules,
    ))
}

#[cfg(test)]
pub(super) fn public_core_capability_names() -> &'static [&'static str] {
    PUBLIC_CORE_CAPABILITIES
}
