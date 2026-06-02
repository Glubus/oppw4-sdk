use super::parse;

#[test]
fn parses_probe_config() {
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

        [entity_counter_probe]
        enabled = true
        interval_ms = 10
        scan_bytes = 99999999
        max_value = 99999999
        max_changes = 999

        [enemy_spawn_probe]
        enabled = true
        max_logs = 99999

        [enemy_stats_probe]
        enabled = true
        max_logs = 99999
        write_stats = true
        hp_multiplier = 999
        attack_multiplier = 2

        [fixed_data_probe]
        enabled = false
        interval_ms = 10
        snapshot_interval_ms = 4000

        [spawn_scaling_probe]
        enabled = false
        interval_ms = 10
        snapshot_interval_ms = 6000
        max_candidates = 999

        [damage_formula_probe]
        enabled = true
        max_logs = 99999

        [rank_runtime]
        easy_s_rankable = true
        shift_count_thresholds = true
        shift_count_row_offset = 1234
        shift_count_rank_row_ids = [35, 77]
        shift_count_source_prefix = [60000, 60000, 48000]
        shift_count_inserted_first = 72000
        shift_count_inserted_second = 73000
        count_threshold_override = [2000, 1200, 1200, 1050, 750]

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

        [rank_helper_hooks]
        enabled = true
        count_enabled = true
        merge_enabled = true
        callsite_enabled = true
        max_logs = 99999

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
    assert!(config.entity_counter_probe.enabled);
    assert_eq!(config.entity_counter_probe.interval_ms, 250);
    assert_eq!(config.entity_counter_probe.scan_bytes, 0x100000);
    assert_eq!(config.entity_counter_probe.max_value, 1_000_000);
    assert_eq!(config.entity_counter_probe.max_changes, 512);
    assert!(config.enemy_spawn_probe.enabled);
    assert_eq!(config.enemy_spawn_probe.max_logs, 4096);
    assert!(config.enemy_stats_probe.enabled);
    assert_eq!(config.enemy_stats_probe.max_logs, 4096);
    assert!(config.enemy_stats_probe.write_stats);
    assert_eq!(config.enemy_stats_probe.hp_multiplier, 100);
    assert_eq!(config.enemy_stats_probe.attack_multiplier, 2);
    assert!(!config.fixed_data_probe.enabled);
    assert_eq!(config.fixed_data_probe.interval_ms, 250);
    assert_eq!(config.fixed_data_probe.snapshot_interval_ms, 4000);
    assert!(!config.spawn_scaling_probe.enabled);
    assert_eq!(config.spawn_scaling_probe.interval_ms, 250);
    assert_eq!(config.spawn_scaling_probe.snapshot_interval_ms, 6000);
    assert_eq!(config.spawn_scaling_probe.max_candidates, 40);
    assert!(config.damage_formula_probe.enabled);
    assert_eq!(config.damage_formula_probe.max_logs, 4096);
    assert!(config.rank_runtime.easy_s_rankable);
    assert!(config.rank_runtime.shift_count_thresholds);
    assert_eq!(config.rank_runtime.shift_count_row_offset, Some(1234));
    assert_eq!(config.rank_runtime.shift_count_rank_row_ids, vec![35, 77]);
    assert_eq!(
        config.rank_runtime.shift_count_source_prefix,
        [60_000, 60_000, 48_000]
    );
    assert_eq!(config.rank_runtime.shift_count_inserted_first, 72_000);
    assert_eq!(
        config.rank_runtime.shift_count_inserted_second,
        Some(73_000)
    );
    assert_eq!(
        config.rank_runtime.count_threshold_override,
        Some([2000, 1200, 1200, 1050, 750])
    );
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
    assert!(config.rank_helper_hooks.enabled);
    assert!(config.rank_helper_hooks.count_enabled);
    assert!(config.rank_helper_hooks.merge_enabled);
    assert!(config.rank_helper_hooks.callsite_enabled);
    assert_eq!(config.rank_helper_hooks.max_logs, 4096);
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
