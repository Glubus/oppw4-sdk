pub(super) const DEFAULT_CONFIG: &str = r#"[config]
type = "sdk_runtime"
version = 1

[difficulty_probe]
enabled = true
interval_ms = 250
dump_reward_row = true
snapshot_interval_ms = 1000

[fixed_data_probe]
enabled = true
interval_ms = 1000
snapshot_interval_ms = 5000

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
