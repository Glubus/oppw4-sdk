pub(super) const DEFAULT_CONFIG: &str = r#"[config]
type = "sdk_runtime"
version = 1

[difficulty_probe]
enabled = true
interval_ms = 250
dump_reward_row = true
snapshot_interval_ms = 1000

[entity_counter_probe]
enabled = false
interval_ms = 500
scan_bytes = 196608
max_value = 5000
max_changes = 48

[enemy_spawn_probe]
enabled = false
max_logs = 128

[enemy_stats_probe]
enabled = false
max_logs = 128
# Read-only by default. Current write filter targets the observed commander /
# officer candidate family: stat3c/stat40 = 390 and source byte00 = 1 or 5.
write_stats = false
hp_multiplier = 1
attack_multiplier = 1

[fixed_data_probe]
enabled = true
interval_ms = 1000
snapshot_interval_ms = 5000

[spawn_scaling_probe]
enabled = true
interval_ms = 1000
snapshot_interval_ms = 10000
max_candidates = 40

[damage_formula_probe]
enabled = false
max_logs = 128

[rank_runtime]
easy_s_rankable = false
shift_count_thresholds = false
# Preferred safe path: patch fixed helper rows by known rank row ids.
# Example: shift_count_rank_row_ids = [35]
shift_count_rank_row_ids = []
# Legacy/diagnostic offset path. Leave unset unless validating with Ghidra.
# shift_count_row_offset = 0
shift_count_source_prefix = [60000, 60000, 48000]
shift_count_inserted_first = 72000
# Optional second inserted threshold for experiments such as [72000, 72000, ...].
# shift_count_inserted_second = 72000
# Optional direct count threshold override for known active rows.
# count_threshold_override = [2000, 1200, 1200, 1050, 750]

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

[rank_helper_hooks]
enabled = false
# Count rank helper is experimental. Keep it off unless we are validating the
# function ABI in Ghidra/runtime logs.
count_enabled = false
# Merge rank helper is read-only and traces FUN_1412dd790 global rank fusion.
merge_enabled = false
# Result-screen callsite diagnostics log after specific CALL return sites. Prefer
# this over helper-entry hooks when validating result-screen ranks.
callsite_enabled = false
max_logs = 256

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
