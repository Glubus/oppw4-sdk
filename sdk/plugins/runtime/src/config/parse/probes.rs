use super::fields::{set_bool, set_u64_min, set_usize_range, u16_values, u32_array, u32_values};
use crate::config::RuntimeConfig;

pub(super) fn parse_all(value: &toml::Value, config: &mut RuntimeConfig) {
    parse_difficulty_probe(value, config);
    parse_entity_counter_probe(value, config);
    parse_fixed_data_probe(value, config);
    parse_spawn_scaling_probe(value, config);
    parse_damage_formula_probe(value, config);
    parse_rank_runtime(value, config);
    parse_player_result_probe(value, config);
    parse_result_probe(value, config);
    parse_rank_threshold_probe(value, config);
    parse_rank_helper_hooks(value, config);
    parse_reward_probe(value, config);
    parse_item_reward_probe(value, config);
    parse_result_state_probe(value, config);
    parse_value_probe(value, config);
}

fn parse_damage_formula_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("damage_formula_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.damage_formula_probe.enabled);
    set_usize_range(
        probe,
        "max_logs",
        1,
        4096,
        &mut config.damage_formula_probe.max_logs,
    );
}

fn parse_entity_counter_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("entity_counter_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.entity_counter_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.entity_counter_probe.interval_ms,
    );
    set_usize_range(
        probe,
        "scan_bytes",
        4096,
        0x100000,
        &mut config.entity_counter_probe.scan_bytes,
    );
    if let Some(value) = probe.get("max_value").and_then(toml::Value::as_integer) {
        config.entity_counter_probe.max_value = (value.max(1) as u32).min(1_000_000);
    }
    set_usize_range(
        probe,
        "max_changes",
        1,
        512,
        &mut config.entity_counter_probe.max_changes,
    );
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

fn parse_spawn_scaling_probe(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value.get("spawn_scaling_probe") else {
        return;
    };
    set_bool(probe, "enabled", &mut config.spawn_scaling_probe.enabled);
    set_u64_min(
        probe,
        "interval_ms",
        250,
        &mut config.spawn_scaling_probe.interval_ms,
    );
    set_u64_min(
        probe,
        "snapshot_interval_ms",
        0,
        &mut config.spawn_scaling_probe.snapshot_interval_ms,
    );
    set_usize_range(
        probe,
        "max_candidates",
        1,
        40,
        &mut config.spawn_scaling_probe.max_candidates,
    );
}

fn parse_rank_runtime(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(runtime) = value.get("rank_runtime") else {
        return;
    };
    set_bool(
        runtime,
        "easy_s_rankable",
        &mut config.rank_runtime.easy_s_rankable,
    );
    set_bool(
        runtime,
        "shift_count_thresholds",
        &mut config.rank_runtime.shift_count_thresholds,
    );
    if let Some(value) = runtime
        .get("shift_count_row_offset")
        .and_then(toml::Value::as_integer)
        .and_then(|value| usize::try_from(value).ok())
    {
        config.rank_runtime.shift_count_row_offset = Some(value.min(0x10_0000));
    }
    if let Some(row_ids) = u16_values(runtime, "shift_count_rank_row_ids") {
        config.rank_runtime.shift_count_rank_row_ids = row_ids;
    }
    if let Some(prefix) = u32_array::<3>(runtime, "shift_count_source_prefix", 1_000_000) {
        config.rank_runtime.shift_count_source_prefix = prefix;
    }
    if let Some(value) = runtime
        .get("shift_count_inserted_first")
        .and_then(toml::Value::as_integer)
    {
        config.rank_runtime.shift_count_inserted_first = (value.max(0) as u32).min(1_000_000);
    }
    if let Some(value) = runtime
        .get("shift_count_inserted_second")
        .and_then(toml::Value::as_integer)
    {
        config.rank_runtime.shift_count_inserted_second =
            Some((value.max(0) as u32).min(1_000_000));
    }
    if let Some(thresholds) = u32_array::<5>(runtime, "count_threshold_override", 1_000_000) {
        config.rank_runtime.count_threshold_override = Some(thresholds);
    }
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

fn parse_rank_helper_hooks(value: &toml::Value, config: &mut RuntimeConfig) {
    let Some(probe) = value
        .get("rank_helper_hooks")
        .or_else(|| value.get("rank_helper_probe"))
    else {
        return;
    };
    set_bool(probe, "enabled", &mut config.rank_helper_hooks.enabled);
    set_bool(
        probe,
        "count_enabled",
        &mut config.rank_helper_hooks.count_enabled,
    );
    set_bool(
        probe,
        "merge_enabled",
        &mut config.rank_helper_hooks.merge_enabled,
    );
    set_bool(
        probe,
        "callsite_enabled",
        &mut config.rank_helper_hooks.callsite_enabled,
    );
    set_usize_range(
        probe,
        "max_logs",
        1,
        4096,
        &mut config.rank_helper_hooks.max_logs,
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
