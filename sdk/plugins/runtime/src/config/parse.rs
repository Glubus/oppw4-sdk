mod fields;
mod probes;

use super::types::RuntimeConfig;

pub(super) fn parse(text: &str) -> Option<RuntimeConfig> {
    let value = text.parse::<toml::Value>().ok()?;
    if config_type(&value)? != "sdk_runtime" {
        return None;
    }

    let mut config = RuntimeConfig::default();
    probes::parse_all(&value, &mut config);
    Some(config)
}

fn config_type(value: &toml::Value) -> Option<&str> {
    value
        .get("config")
        .and_then(|config| config.get("type"))
        .and_then(toml::Value::as_str)
}
