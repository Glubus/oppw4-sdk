use std::{fs, path::Path};

use plugin_sdk::HostApi;

mod cycle;
mod fx;
mod plugin;

pub(crate) use cycle::{CycleConfig, CycleMode};
pub(crate) use fx::{FxConfig, TargetMode};
pub(crate) use plugin::{InstallMode, PluginConfig, StatusGate, TriggerMode};

const DEFAULT_CONFIG: &str = r#"[config]
type = "fx_director"
version = 1

[debug]
observe_effect_ids = false
observe_character_probe = false
"#;

pub(crate) fn load_plugin_config(host: HostApi<'_>) -> PluginConfig {
    let Some(root) = host.paths().config_root() else {
        return PluginConfig::default();
    };
    let path = root.join("config.toml");
    ensure_default_config(&path);
    let Ok(text) = fs::read_to_string(path) else {
        return PluginConfig::default();
    };
    parse_plugin_config(&text).unwrap_or_default()
}

fn ensure_default_config(path: &Path) {
    if path.is_file() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, DEFAULT_CONFIG);
}

fn parse_plugin_config(text: &str) -> Option<PluginConfig> {
    let value = text.parse::<toml::Value>().ok()?;
    let config_type = value
        .get("config")
        .and_then(|config| config.get("type"))
        .and_then(toml::Value::as_str)?;
    if config_type != "fx_director" {
        return None;
    }

    let mut config = PluginConfig::default();
    if let Some(plugin) = value.get("plugin") {
        if let Some(mode) = plugin.get("install_mode").and_then(toml::Value::as_str) {
            config.install_mode = parse_install_mode(mode);
        }
    }
    if let Some(debug) = value.get("debug") {
        if let Some(observe_effect_ids) = debug
            .get("observe_effect_ids")
            .and_then(toml::Value::as_bool)
        {
            config.debug.observe_effect_ids = observe_effect_ids;
        }
        if let Some(observe_character_probe) = debug
            .get("observe_character_probe")
            .and_then(toml::Value::as_bool)
        {
            config.debug.observe_character_probe = observe_character_probe;
        }
    }
    Some(config)
}

fn parse_install_mode(value: &str) -> InstallMode {
    if value.eq_ignore_ascii_case("scan_only") {
        InstallMode::ScanOnly
    } else if value.eq_ignore_ascii_case("local_player_probe") {
        InstallMode::LocalPlayerProbe
    } else {
        InstallMode::Patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn plugin_debug_config_is_separate_from_fx_definitions() {
        let config = parse_plugin_config(
            r#"
            [config]
            type = "fx_director"
            version = 1

            [debug]
            observe_effect_ids = true
            observe_character_probe = true
            "#,
        )
        .expect("config");

        assert!(config.debug.observe_effect_ids);
        assert!(config.debug.observe_character_probe);
        assert_eq!(config.install_mode, InstallMode::Patch);
    }

    #[test]
    fn creates_default_config_when_missing() {
        let root = temp_root("fx-director-config");
        let path = root.join("config.toml");

        ensure_default_config(&path);

        let text = fs::read_to_string(&path).expect("default config");
        assert!(text.contains("type = \"fx_director\""));
        assert_eq!(
            parse_plugin_config(&text).expect("config"),
            PluginConfig::default()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn does_not_overwrite_existing_config() {
        let root = temp_root("fx-director-config-existing");
        fs::create_dir_all(&root).expect("temp config dir");
        let path = root.join("config.toml");
        fs::write(&path, "[config]\ntype = \"custom\"\n").expect("existing config");

        ensure_default_config(&path);

        assert_eq!(
            fs::read_to_string(&path).expect("config"),
            "[config]\ntype = \"custom\"\n"
        );
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
