use std::{collections::HashSet, fs, path::PathBuf};

use crate::runtime::manifest::PluginManifest;

use super::{
    manifests::{plugin_dirs, reject_duplicate_manifests},
    rules::{capabilities_available, dependencies_loaded},
    services::{
        capabilities_available_for_manifest, public_core_capability_names, sdk_service_manifests,
    },
};

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
    let available = public_core_capability_names()
        .iter()
        .map(|capability| (*capability).to_string())
        .collect::<HashSet<_>>();

    assert!(capabilities_available(
        &["mod.discovery".to_string(), "memory.read".to_string()],
        &available
    ));
    assert!(!capabilities_available(
        &["linkdata.patch".to_string()],
        &available
    ));
    assert!(capabilities_available(
        &["files.virtualize".to_string()],
        &available
    ));
    assert!(capabilities_available(
        &["registry.module".to_string()],
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
    let available = public_core_capability_names()
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
    fs::write(sdk_root.join("data.dll"), []).expect("data dll");
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
            "sdk_data",
            "sdk_runtime",
            "sdk_debug",
            "sdk_overlay",
            "sdk_linkdata",
            "sdk_rdb"
        ]
    );
    assert_eq!(manifests[0].entry_path, sdk_root.join("data.dll"));
    assert!(manifests[0]
        .capabilities_required
        .iter()
        .any(|capability| capability == "registry.module"));
    assert_eq!(manifests[0].registry_modules, ["sdk.character"]);
    assert_eq!(manifests[1].entry_path, sdk_root.join("runtime.dll"));
    assert!(manifests[1]
        .capabilities_required
        .iter()
        .any(|capability| capability == "config.schema"));
    assert_eq!(manifests[4].entry_path, sdk_root.join("linkdata.dll"));
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
        registry_modules: Vec::new(),
        capabilities_required: Vec::new(),
        capabilities_provided: Vec::new(),
        mods_root: root.join("mods"),
        entry_path: root.join(format!("{id}.dll")),
        log_root: root.join("logs"),
        root,
    }
}
