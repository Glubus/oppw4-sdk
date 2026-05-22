use super::types::RuntimeConfig;

pub(super) fn parse(text: &str) -> Option<RuntimeConfig> {
    let value = text.parse::<toml::Value>().ok()?;
    if config_type(&value)? != "sdk_runtime" {
        return None;
    }

    let mut config = RuntimeConfig::default();
    parse_difficulty_probe(&value, &mut config);
    parse_player_result_probe(&value, &mut config);
    parse_result_probe(&value, &mut config);
    parse_rank_threshold_probe(&value, &mut config);
    parse_reward_probe(&value, &mut config);
    parse_item_reward_probe(&value, &mut config);
    parse_result_state_probe(&value, &mut config);
    parse_value_probe(&value, &mut config);
    Some(config)
}

fn config_type(value: &toml::Value) -> Option<&str> {
    value
        .get("config")
        .and_then(|config| config.get("type"))
        .and_then(toml::Value::as_str)
}

fn parse_difficulty_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("difficulty_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.difficulty_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        50,
        &mut config.difficulty_probe.interval_ms,
    );
    set_bool(
        probe,
        "dump_reward_row",
        &mut config.difficulty_probe.dump_reward_row,
    );
    set_u64_min(
        probe,
        "snapshot_interval_ms",
        0,
        &mut config.difficulty_probe.snapshot_interval_ms,
    );
}

fn parse_player_result_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("player_result_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.player_result_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.player_result_probe.interval_ms,
    );
    set_u64_min(
        probe,
        "snapshot_interval_ms",
        0,
        &mut config.player_result_probe.snapshot_interval_ms,
    );
}

fn parse_result_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("result_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.result_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.result_probe.interval_ms,
    );
    set_usize_range(
        probe,
        "result_area_bytes",
        64,
        4096,
        &mut config.result_probe.result_area_bytes,
    );
    set_usize_range(
        probe,
        "max_changed_words",
        1,
        64,
        &mut config.result_probe.max_changed_words,
    );
}

fn parse_rank_threshold_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("rank_threshold_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.rank_threshold_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.rank_threshold_probe.interval_ms,
    );
    set_u64_min(
        probe,
        "snapshot_interval_ms",
        0,
        &mut config.rank_threshold_probe.snapshot_interval_ms,
    );
}

fn parse_reward_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("reward_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.reward_probe.enabled);
    set_usize_range(
        probe,
        "max_logs",
        1,
        4096,
        &mut config.reward_probe.max_logs,
    );
}

fn parse_item_reward_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("item_reward_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.item_reward_probe.enabled);
    set_usize_range(
        probe,
        "max_logs",
        1,
        4096,
        &mut config.item_reward_probe.max_logs,
    );
    set_usize_range(
        probe,
        "max_entries",
        1,
        40,
        &mut config.item_reward_probe.max_entries,
    );
}

fn parse_result_state_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("result_state_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.result_state_probe.enabled);
    set_usize_range(
        probe,
        "max_logs",
        1,
        4096,
        &mut config.result_state_probe.max_logs,
    );
    set_usize_range(
        probe,
        "max_events",
        1,
        128,
        &mut config.result_state_probe.max_events,
    );
}

fn parse_value_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("value_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.value_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.value_probe.interval_ms,
    );
    set_usize_range(
        probe,
        "scan_bytes",
        4096,
        0x100000,
        &mut config.value_probe.scan_bytes,
    );
    set_usize_range(probe, "max_hits", 1, 256, &mut config.value_probe.max_hits);
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

fn set_bool(table: &toml::Value, key: &str, output: &mut bool) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_bool) {
        *output = value;
    }
}

fn set_u64_min(table: &toml::Value, key: &str, min: i64, output: &mut u64) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_integer) {
        *output = value.max(min) as u64;
    }
}

fn set_usize_range(table: &toml::Value, key: &str, min: i64, max: usize, output: &mut usize) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_integer) {
        *output = (value.max(min) as usize).min(max);
    }
}
