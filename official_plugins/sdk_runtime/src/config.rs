use std::{fs, path::Path};

use plugin_sdk::HostApi;

const DEFAULT_CONFIG: &str = r#"[config]
type = "sdk_runtime"
version = 1

[difficulty_probe]
enabled = true
interval_ms = 250
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DifficultyProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
}

impl Default for DifficultyProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 250,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeConfig {
    pub(crate) difficulty_probe: DifficultyProbeConfig,
}

pub(crate) fn load(host: HostApi<'_>) -> RuntimeConfig {
    let Some(root) = host.paths().config_root() else {
        return RuntimeConfig::default();
    };
    let path = root.join("config.toml");
    ensure_default_config(&path);
    let Ok(text) = fs::read_to_string(path) else {
        return RuntimeConfig::default();
    };
    parse(&text).unwrap_or_default()
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

fn parse(text: &str) -> Option<RuntimeConfig> {
    let value = text.parse::<toml::Value>().ok()?;
    let config_type = value
        .get("config")
        .and_then(|config| config.get("type"))
        .and_then(toml::Value::as_str)?;
    if config_type != "sdk_runtime" {
        return None;
    }

    let mut config = RuntimeConfig::default();
    if let Some(probe) = value.get("difficulty_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.difficulty_probe.enabled = enabled;
        }
        if let Some(interval_ms) = probe.get("interval_ms").and_then(toml::Value::as_integer) {
            config.difficulty_probe.interval_ms = interval_ms.max(50) as u64;
        }
    }
    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_difficulty_probe_config() {
        let config = parse(
            r#"
            [config]
            type = "sdk_runtime"
            version = 1

            [difficulty_probe]
            enabled = false
            interval_ms = 10
            "#,
        )
        .expect("config");

        assert!(!config.difficulty_probe.enabled);
        assert_eq!(config.difficulty_probe.interval_ms, 50);
    }

    #[test]
    fn rejects_wrong_config_type() {
        assert_eq!(parse("[config]\ntype = \"fx_director\"\n"), None);
    }
}
