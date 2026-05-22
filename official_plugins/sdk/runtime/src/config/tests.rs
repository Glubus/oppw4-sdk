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

        [fixed_data_probe]
        enabled = false
        interval_ms = 10
        snapshot_interval_ms = 4000

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
    assert!(!config.fixed_data_probe.enabled);
    assert_eq!(config.fixed_data_probe.interval_ms, 250);
    assert_eq!(config.fixed_data_probe.snapshot_interval_ms, 4000);
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
