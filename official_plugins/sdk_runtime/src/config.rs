use std::{fs, path::Path};

use plugin_sdk::HostApi;

const DEFAULT_CONFIG: &str = r#"[config]
type = "sdk_runtime"
version = 1

[difficulty_probe]
enabled = true
interval_ms = 250
dump_reward_row = true
snapshot_interval_ms = 1000

[player_result_probe]
enabled = true
interval_ms = 500
snapshot_interval_ms = 1000

[result_probe]
enabled = true
interval_ms = 1000
result_area_bytes = 512
max_changed_words = 16

[rank_threshold_probe]
enabled = true
interval_ms = 1000
snapshot_interval_ms = 1000

[reward_probe]
enabled = true
max_logs = 64

[item_reward_probe]
enabled = true
max_logs = 64
max_entries = 40

[result_state_probe]
enabled = true
max_logs = 128
max_events = 24

[value_probe]
enabled = true
interval_ms = 1000
scan_bytes = 196608
max_hits = 64
values = [1247, 1500, 170, 30, 200, 73, 70, 58, 26, 5, 31, 487000, 18250, 992250, 221650]
"#;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DifficultyProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) dump_reward_row: bool,
    pub(crate) snapshot_interval_ms: u64,
}

impl Default for DifficultyProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 250,
            dump_reward_row: true,
            snapshot_interval_ms: 1000,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeConfig {
    pub(crate) difficulty_probe: DifficultyProbeConfig,
    pub(crate) player_result_probe: PlayerResultProbeConfig,
    pub(crate) result_probe: ResultProbeConfig,
    pub(crate) rank_threshold_probe: RankThresholdProbeConfig,
    pub(crate) reward_probe: RewardProbeConfig,
    pub(crate) item_reward_probe: ItemRewardProbeConfig,
    pub(crate) result_state_probe: ResultStateProbeConfig,
    pub(crate) value_probe: ValueProbeConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayerResultProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) snapshot_interval_ms: u64,
}

impl Default for PlayerResultProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 500,
            snapshot_interval_ms: 1000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) result_area_bytes: usize,
    pub(crate) max_changed_words: usize,
}

impl Default for ResultProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 1000,
            result_area_bytes: 512,
            max_changed_words: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RankThresholdProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) snapshot_interval_ms: u64,
}

impl Default for RankThresholdProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 1000,
            snapshot_interval_ms: 1000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RewardProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) max_logs: usize,
}

impl Default for RewardProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_logs: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemRewardProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) max_logs: usize,
    pub(crate) max_entries: usize,
}

impl Default for ItemRewardProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_logs: 64,
            max_entries: 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultStateProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) max_logs: usize,
    pub(crate) max_events: usize,
}

impl Default for ResultStateProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_logs: 128,
            max_events: 24,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValueProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) scan_bytes: usize,
    pub(crate) max_hits: usize,
    pub(crate) values: Vec<u32>,
}

impl Default for ValueProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 1000,
            scan_bytes: 0x30000,
            max_hits: 64,
            values: vec![
                1247, 1500, 170, 30, 200, 73, 70, 58, 26, 5, 31, 487_000, 18_250, 992_250, 221_650,
            ],
        }
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
        if let Some(enabled) = probe.get("dump_reward_row").and_then(toml::Value::as_bool) {
            config.difficulty_probe.dump_reward_row = enabled;
        }
        if let Some(interval_ms) = probe
            .get("snapshot_interval_ms")
            .and_then(toml::Value::as_integer)
        {
            config.difficulty_probe.snapshot_interval_ms = interval_ms.max(0) as u64;
        }
    }
    if let Some(probe) = value.get("player_result_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.player_result_probe.enabled = enabled;
        }
        if let Some(interval_ms) = probe.get("interval_ms").and_then(toml::Value::as_integer) {
            config.player_result_probe.interval_ms = interval_ms.max(250) as u64;
        }
        if let Some(interval_ms) = probe
            .get("snapshot_interval_ms")
            .and_then(toml::Value::as_integer)
        {
            config.player_result_probe.snapshot_interval_ms = interval_ms.max(0) as u64;
        }
    }
    if let Some(probe) = value.get("result_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.result_probe.enabled = enabled;
        }
        if let Some(interval_ms) = probe.get("interval_ms").and_then(toml::Value::as_integer) {
            config.result_probe.interval_ms = interval_ms.max(250) as u64;
        }
        if let Some(bytes) = probe
            .get("result_area_bytes")
            .and_then(toml::Value::as_integer)
        {
            config.result_probe.result_area_bytes = (bytes.max(64) as usize).min(4096);
        }
        if let Some(words) = probe
            .get("max_changed_words")
            .and_then(toml::Value::as_integer)
        {
            config.result_probe.max_changed_words = (words.max(1) as usize).min(64);
        }
    }
    if let Some(probe) = value.get("rank_threshold_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.rank_threshold_probe.enabled = enabled;
        }
        if let Some(interval_ms) = probe.get("interval_ms").and_then(toml::Value::as_integer) {
            config.rank_threshold_probe.interval_ms = interval_ms.max(250) as u64;
        }
        if let Some(interval_ms) = probe
            .get("snapshot_interval_ms")
            .and_then(toml::Value::as_integer)
        {
            config.rank_threshold_probe.snapshot_interval_ms = interval_ms.max(0) as u64;
        }
    }
    if let Some(probe) = value.get("reward_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.reward_probe.enabled = enabled;
        }
        if let Some(max_logs) = probe.get("max_logs").and_then(toml::Value::as_integer) {
            config.reward_probe.max_logs = (max_logs.max(1) as usize).min(4096);
        }
    }
    if let Some(probe) = value.get("item_reward_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.item_reward_probe.enabled = enabled;
        }
        if let Some(max_logs) = probe.get("max_logs").and_then(toml::Value::as_integer) {
            config.item_reward_probe.max_logs = (max_logs.max(1) as usize).min(4096);
        }
        if let Some(max_entries) = probe.get("max_entries").and_then(toml::Value::as_integer) {
            config.item_reward_probe.max_entries = (max_entries.max(1) as usize).min(40);
        }
    }
    if let Some(probe) = value.get("result_state_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.result_state_probe.enabled = enabled;
        }
        if let Some(max_logs) = probe.get("max_logs").and_then(toml::Value::as_integer) {
            config.result_state_probe.max_logs = (max_logs.max(1) as usize).min(4096);
        }
        if let Some(max_events) = probe.get("max_events").and_then(toml::Value::as_integer) {
            config.result_state_probe.max_events = (max_events.max(1) as usize).min(128);
        }
    }
    if let Some(probe) = value.get("value_probe") {
        if let Some(enabled) = probe.get("enabled").and_then(toml::Value::as_bool) {
            config.value_probe.enabled = enabled;
        }
        if let Some(interval_ms) = probe.get("interval_ms").and_then(toml::Value::as_integer) {
            config.value_probe.interval_ms = interval_ms.max(250) as u64;
        }
        if let Some(bytes) = probe.get("scan_bytes").and_then(toml::Value::as_integer) {
            config.value_probe.scan_bytes = (bytes.max(4096) as usize).min(0x100000);
        }
        if let Some(max_hits) = probe.get("max_hits").and_then(toml::Value::as_integer) {
            config.value_probe.max_hits = (max_hits.max(1) as usize).min(256);
        }
        if let Some(values) = probe.get("values").and_then(toml::Value::as_array) {
            let parsed = values
                .iter()
                .filter_map(toml::Value::as_integer)
                .filter_map(|value| u32::try_from(value).ok())
                .collect::<Vec<_>>();
            if !parsed.is_empty() {
                config.value_probe.values = parsed;
            }
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
            dump_reward_row = false
            snapshot_interval_ms = 5000

            [player_result_probe]
            enabled = false
            interval_ms = 10
            snapshot_interval_ms = 2000

            [result_probe]
            enabled = false
            interval_ms = 10
            result_area_bytes = 99999
            max_changed_words = 999

            [rank_threshold_probe]
            enabled = false
            interval_ms = 10
            snapshot_interval_ms = 3000

            [reward_probe]
            enabled = false
            max_logs = 99999

            [item_reward_probe]
            enabled = false
            max_logs = 99999
            max_entries = 999

            [result_state_probe]
            enabled = false
            max_logs = 99999
            max_events = 999

            [value_probe]
            enabled = false
            interval_ms = 10
            scan_bytes = 99999999
            max_hits = 999
            values = [1, 2, 3]
            "#,
        )
        .expect("config");

        assert!(!config.difficulty_probe.enabled);
        assert_eq!(config.difficulty_probe.interval_ms, 50);
        assert!(!config.difficulty_probe.dump_reward_row);
        assert_eq!(config.difficulty_probe.snapshot_interval_ms, 5000);
        assert!(!config.player_result_probe.enabled);
        assert_eq!(config.player_result_probe.interval_ms, 250);
        assert_eq!(config.player_result_probe.snapshot_interval_ms, 2000);
        assert!(!config.result_probe.enabled);
        assert_eq!(config.result_probe.interval_ms, 250);
        assert_eq!(config.result_probe.result_area_bytes, 4096);
        assert_eq!(config.result_probe.max_changed_words, 64);
        assert!(!config.rank_threshold_probe.enabled);
        assert_eq!(config.rank_threshold_probe.interval_ms, 250);
        assert_eq!(config.rank_threshold_probe.snapshot_interval_ms, 3000);
        assert!(!config.reward_probe.enabled);
        assert_eq!(config.reward_probe.max_logs, 4096);
        assert!(!config.item_reward_probe.enabled);
        assert_eq!(config.item_reward_probe.max_logs, 4096);
        assert_eq!(config.item_reward_probe.max_entries, 40);
        assert!(!config.result_state_probe.enabled);
        assert_eq!(config.result_state_probe.max_logs, 4096);
        assert_eq!(config.result_state_probe.max_events, 128);
        assert!(!config.value_probe.enabled);
        assert_eq!(config.value_probe.interval_ms, 250);
        assert_eq!(config.value_probe.scan_bytes, 0x100000);
        assert_eq!(config.value_probe.max_hits, 256);
        assert_eq!(config.value_probe.values, vec![1, 2, 3]);
    }

    #[test]
    fn rejects_wrong_config_type() {
        assert_eq!(parse("[config]\ntype = \"fx_director\"\n"), None);
    }
}
