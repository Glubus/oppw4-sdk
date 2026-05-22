use super::fields::{set_bool, set_u64_min, set_usize_range, u32_values};
use crate::config::RuntimeConfig;

pub(super) fn parse_all(value: &toml::Value, config: &mut RuntimeConfig) {
    parse_difficulty_probe(value, config);
    parse_fixed_data_probe(value, config);
    parse_player_result_probe(value, config);
    parse_result_probe(value, config);
    parse_rank_threshold_probe(value, config);
    parse_reward_probe(value, config);
    parse_item_reward_probe(value, config);
    parse_result_state_probe(value, config);
    parse_value_probe(value, config);
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

fn parse_fixed_data_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("fixed_data_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.fixed_data_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.fixed_data_probe.interval_ms,
    );
    set_u64_min(
        probe,
        "snapshot_interval_ms",
        0,
        &mut config.fixed_data_probe.snapshot_interval_ms,
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
    if let Some(values) = u32_values(probe, "values") {
        config.value_probe.values = values;
    }
}
