use std::{fs, path::PathBuf};

use plugin_sdk::HostApi;

use crate::PLUGIN_ID;

pub(crate) const DEFAULT_CONFIG: &str = r#"[config]
type = "sdk_overlay"
version = 1

[overlay]
enabled = false
backend = "auto"
poll_interval_ms = 1000

[debug]
log_renderer_probe = true
"#;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OverlayConfig {
    pub(crate) enabled: bool,
    pub(crate) backend: OverlayBackend,
    pub(crate) poll_interval_ms: u64,
    pub(crate) log_renderer_probe: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: OverlayBackend::Auto,
            poll_interval_ms: 1000,
            log_renderer_probe: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverlayBackend {
    Auto,
    Dxgi,
    Disabled,
}

pub(crate) fn register_schema(host: HostApi<'_>) {
    let _ = host
        .configs()
        .register_schema(PLUGIN_ID, "config.toml", DEFAULT_CONFIG)
        .map_err(|error| {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("sdk_overlay config schema register failed: {error}"),
            );
        });
}

pub(crate) fn ensure_config(host: HostApi<'_>) -> PathBuf {
    let root = host
        .paths()
        .config_root()
        .unwrap_or_else(|| PathBuf::from("plugins/configs"));
    let plugin_root = root.join(PLUGIN_ID);
    let path = plugin_root.join("config.toml");
    if !path.exists() {
        let _ = fs::create_dir_all(&plugin_root);
        let _ = fs::write(&path, DEFAULT_CONFIG);
    }
    path
}

pub(crate) fn load(path: &PathBuf) -> OverlayConfig {
    let Ok(text) = fs::read_to_string(path) else {
        return OverlayConfig::default();
    };
    parse(&text).unwrap_or_default()
}

fn parse(text: &str) -> Option<OverlayConfig> {
    let value = text.parse::<toml::Value>().ok()?;
    if value
        .get("config")?
        .get("type")?
        .as_str()?
        .eq_ignore_ascii_case("sdk_overlay")
    {
        let mut config = OverlayConfig::default();
        if let Some(overlay) = value.get("overlay") {
            if let Some(enabled) = overlay.get("enabled").and_then(toml::Value::as_bool) {
                config.enabled = enabled;
            }
            if let Some(backend) = overlay.get("backend").and_then(toml::Value::as_str) {
                config.backend = parse_backend(backend);
            }
            if let Some(interval) = overlay
                .get("poll_interval_ms")
                .and_then(toml::Value::as_integer)
            {
                config.poll_interval_ms = interval.max(250) as u64;
            }
        }
        if let Some(debug) = value.get("debug") {
            if let Some(log) = debug
                .get("log_renderer_probe")
                .and_then(toml::Value::as_bool)
            {
                config.log_renderer_probe = log;
            }
        }
        Some(config)
    } else {
        None
    }
}

fn parse_backend(value: &str) -> OverlayBackend {
    match value.to_ascii_lowercase().as_str() {
        "dxgi" => OverlayBackend::Dxgi,
        "disabled" | "none" => OverlayBackend::Disabled,
        _ => OverlayBackend::Auto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_overlay_config() {
        let config = parse(
            r#"
            [config]
            type = "sdk_overlay"
            [overlay]
            enabled = true
            backend = "dxgi"
            poll_interval_ms = 1
            [debug]
            log_renderer_probe = false
            "#,
        )
        .expect("config");

        assert!(config.enabled);
        assert_eq!(config.backend, OverlayBackend::Dxgi);
        assert_eq!(config.poll_interval_ms, 250);
        assert!(!config.log_renderer_probe);
    }
}
