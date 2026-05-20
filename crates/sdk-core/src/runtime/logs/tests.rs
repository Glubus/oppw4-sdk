use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use plugin_abi::cstring_lossy;

use super::LogRouter;

#[test]
fn plugin_logs_are_routed_to_registered_plugin_folder() {
    let root = temp_root("plugin-log-routing");
    let mut router = LogRouter::new(
        Some("2026-05-16-201122".to_string()),
        root.join("_oppw4").join("logs").join("mods"),
    );
    router.register(
        "skin_patcher".to_string(),
        root.join("skin_patcher").join("logs"),
    );
    router.register("fx_tools".to_string(), root.join("fx_tools").join("logs"));
    let skin = cstring_lossy("skin_patcher");
    let fx = cstring_lossy("fx_tools");

    router
        .route_plugin(&skin, &cstring_lossy("skin online"))
        .expect("skin log");
    router
        .route_plugin(&fx, &cstring_lossy("fx online"))
        .expect("fx log");

    assert_eq!(
        fs::read_to_string(
            root.join("skin_patcher")
                .join("logs")
                .join("2026-05-16-201122.log")
        )
        .expect("skin log file"),
        "[2026-05-16 20:11:22] skin online\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("fx_tools")
                .join("logs")
                .join("2026-05-16-201122.log")
        )
        .expect("fx log file"),
        "[2026-05-16 20:11:22] fx online\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn plugin_log_file_names_are_sanitized() {
    let root = temp_root("plugin-log-sanitize");
    let mut router = LogRouter::new(
        Some("2026-05-16-201122".to_string()),
        root.join("_oppw4").join("logs").join("mods"),
    );
    router.register(
        "../skin patcher.dll".to_string(),
        root.join("skin_patcher_dll").join("logs"),
    );
    let plugin = cstring_lossy("../skin patcher.dll");

    router
        .route_plugin(&plugin, &cstring_lossy("clean path"))
        .expect("sanitized log");

    assert_eq!(
        fs::read_to_string(
            root.join("skin_patcher_dll")
                .join("logs")
                .join("2026-05-16-201122.log")
        )
        .expect("sanitized log file"),
        "[2026-05-16 20:11:22] clean path\n"
    );
    assert!(!root.join("..").join("2026-05-16-201122.log").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn mod_logs_are_grouped_by_mod_id() {
    let root = temp_root("mod-log-routing");
    let mod_log_root = root.join("_oppw4").join("logs").join("mods");
    let mut router = LogRouter::new(Some("2026-05-16-201122".to_string()), mod_log_root.clone());

    router
        .route_mod(
            "fx cycle test",
            "lua mod log id=fx_cycle_test level=info message=online",
        )
        .expect("mod log");

    assert_eq!(
        fs::read_to_string(
            mod_log_root
                .join("fx_cycle_test")
                .join("2026-05-16-201122.log")
        )
        .expect("mod log file"),
        "[2026-05-16 20:11:22] lua mod log id=fx_cycle_test level=info message=online\n"
    );
    let _ = fs::remove_dir_all(root);
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
}
