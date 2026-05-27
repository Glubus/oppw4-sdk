use std::{fs, path::PathBuf};

use plugin_sdk::HostApi;

use crate::PLUGIN_ID;

pub(crate) const DEFAULT_SCRIPT: &str = r#"enabled = false
interval_ms = 500

# Watches log only when the value changes.
# address can be:
#   "module+0x1e5ec04"
#   0x141e5ec04
#   { base = "module+0x1eba750", offsets = [0x18, 0x28, 0x1d756] }
#
# [[watches]]
# id = "cached_difficulty"
# type = "u32"
# address = "module+0x1e5ec04"
#
# [[scans]]
# id = "visible_souls"
# type = "u32"
# start = "module+0x100000"
# bytes = 0x200000
# values = [1, 2]
# max_hits = 32
"#;

pub(crate) fn register_schema(host: HostApi<'_>) {
    let _ = host
        .configs()
        .register_schema(PLUGIN_ID, "debug.toml", DEFAULT_SCRIPT)
        .map_err(|error| {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("sdk_debug config schema register failed: {error}"),
            );
        });
}

pub(crate) fn ensure_debug_script(host: HostApi<'_>) -> PathBuf {
    let root = host
        .paths()
        .config_root()
        .unwrap_or_else(|| PathBuf::from("plugins/configs"));
    let plugin_root = root.join(PLUGIN_ID);
    let path = plugin_root.join("debug.toml");
    if !path.exists() {
        let _ = fs::create_dir_all(&plugin_root);
        let _ = fs::write(&path, DEFAULT_SCRIPT);
    }
    path
}
