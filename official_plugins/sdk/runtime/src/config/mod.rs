mod defaults;
mod parse;
mod types;

use std::{fs, path::Path};

use plugin_sdk::HostApi;

use defaults::DEFAULT_CONFIG;
use parse::parse;

pub(crate) use types::{
    DamageFormulaProbeConfig, DifficultyProbeConfig, EntityCounterProbeConfig,
    FixedDataProbeConfig, ItemRewardProbeConfig, PlayerResultProbeConfig, RankHelperProbeConfig,
    RankRuntimeConfig, RankThresholdProbeConfig, ResultProbeConfig, ResultStateProbeConfig,
    RewardProbeConfig, RuntimeConfig, SpawnScalingProbeConfig, ValueProbeConfig,
};

pub(crate) fn register_schema(host: HostApi<'_>) {
    if let Err(error) = host
        .configs()
        .register_schema("sdk_runtime", "config.toml", DEFAULT_CONFIG)
    {
        let _ = host.log().write(
            "sdk_runtime",
            format!("sdk_runtime config schema register failed: {error}"),
        );
    }
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
