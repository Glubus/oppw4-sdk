use std::{
    fs,
    path::{Path, PathBuf},
};

use super::PluginManifest;

#[test]
fn manifest_points_entry_and_logs_inside_plugin_folder() {
    let root = PathBuf::from(r"D:\Game\OPPW4\plugins\skin_patcher");
    let manifest = PluginManifest::parse(
        &root,
        r#"
                [plugin]
                id = "skin_patcher"
                version = "0.1.0"
                entry = "skin_patcher.dll"

                [dependencies]
                plugins = ["sdk_core"]

                [lua]
                modules = ["skin_patcher"]

                [capabilities]
                requires = ["lua.module", "rdb.patch"]
                provides = ["std.character.extend"]
            "#,
    )
    .expect("manifest");

    assert_eq!(manifest.id, "skin_patcher");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.dependencies, ["sdk_core"]);
    assert_eq!(manifest.lua_modules, ["skin_patcher"]);
    assert_eq!(manifest.capabilities_required, ["lua.module", "rdb.patch"]);
    assert_eq!(manifest.capabilities_provided, ["std.character.extend"]);
    assert_eq!(manifest.root, root);
    assert_eq!(manifest.mods_root, root.join("mods"));
    assert_eq!(manifest.entry_path, root.join("skin_patcher.dll"));
    assert_eq!(manifest.log_root, root.join("logs"));
}

#[test]
fn manifest_rejects_entry_with_path_segments() {
    let error = PluginManifest::parse(
        Path::new(r"D:\Game\OPPW4\plugins\bad"),
        r#"
                [plugin]
                id = "bad"
                version = "0.1.0"
                entry = "bin/bad.dll"
            "#,
    )
    .expect_err("entry should be rejected");

    assert!(error.contains("file name only"));
}

#[test]
fn manifest_requires_version() {
    let error = PluginManifest::parse(
        Path::new(r"D:\Game\OPPW4\plugins\bad"),
        r#"
                [plugin]
                id = "bad"
                entry = "bad.dll"
            "#,
    )
    .expect_err("version should be required");

    assert!(error.contains("plugin.version"));
}

#[test]
fn manifest_rejects_invalid_capability_names() {
    let error = PluginManifest::parse(
        Path::new(r"D:\Game\OPPW4\plugins\bad"),
        r#"
                [plugin]
                id = "bad"
                version = "0.1.0"
                entry = "bad.dll"

                [capabilities]
                requires = ["hooks/evil"]
            "#,
    )
    .expect_err("capability should be rejected");

    assert!(error.contains("capability name is invalid"));
}

#[test]
fn manifest_file_is_named_plugin_toml() {
    let root = temp_root("plugin-toml");
    fs::create_dir_all(&root).expect("temp plugin dir");
    fs::write(
        root.join("plugin.toml"),
        r#"
                [plugin]
                id = "skin_patcher"
                version = "0.1.0"
                entry = "skin_patcher.dll"
            "#,
    )
    .expect("plugin manifest");

    let manifest = PluginManifest::read_from_dir(&root).expect("manifest");

    assert_eq!(manifest.id, "skin_patcher");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn creates_plugin_toml_from_matching_dll_when_missing() {
    let root = temp_root("auto-plugin-toml").join("fx_director");
    fs::create_dir_all(&root).expect("temp plugin dir");
    fs::write(root.join("fx_director.dll"), []).expect("plugin dll");

    let manifest = PluginManifest::read_from_dir(&root).expect("manifest");

    assert_eq!(manifest.id, "fx_director");
    assert_eq!(manifest.version, "0.2.0");
    assert_eq!(manifest.entry_path, root.join("fx_director.dll"));
    assert!(root.join("plugin.toml").is_file());
    let _ = fs::remove_dir_all(root.parent().expect("temp root"));
}

#[test]
fn does_not_create_plugin_toml_when_entry_is_ambiguous() {
    let root = temp_root("ambiguous-plugin-toml");
    fs::create_dir_all(&root).expect("temp plugin dir");
    fs::write(root.join("a.dll"), []).expect("first dll");
    fs::write(root.join("b.dll"), []).expect("second dll");

    assert!(PluginManifest::read_from_dir(&root).is_none());
    assert!(!root.join("plugin.toml").is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_mod_toml_is_not_a_plugin_manifest_without_a_dll() {
    let root = temp_root("mod-toml");
    fs::create_dir_all(&root).expect("temp plugin dir");
    fs::write(
        root.join("mod.toml"),
        r#"
                [plugin]
                id = "skin_patcher"
                version = "0.1.0"
                entry = "skin_patcher.dll"
            "#,
    )
    .expect("legacy manifest");

    assert!(PluginManifest::read_from_dir(&root).is_none());
    let _ = fs::remove_dir_all(root);
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
}
