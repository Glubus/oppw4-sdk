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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RuntimeConfig {
    pub(crate) difficulty_probe: DifficultyProbeConfig,
    pub(crate) entity_counter_probe: EntityCounterProbeConfig,
    pub(crate) fixed_data_probe: FixedDataProbeConfig,
    pub(crate) spawn_scaling_probe: SpawnScalingProbeConfig,
    pub(crate) damage_formula_probe: DamageFormulaProbeConfig,
    pub(crate) rank_runtime: RankRuntimeConfig,
    pub(crate) player_result_probe: PlayerResultProbeConfig,
    pub(crate) result_probe: ResultProbeConfig,
    pub(crate) rank_threshold_probe: RankThresholdProbeConfig,
    pub(crate) rank_helper_hooks: RankHelperHooksConfig,
    pub(crate) reward_probe: RewardProbeConfig,
    pub(crate) item_reward_probe: ItemRewardProbeConfig,
    pub(crate) result_state_probe: ResultStateProbeConfig,
    pub(crate) value_probe: ValueProbeConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntityCounterProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) scan_bytes: usize,
    pub(crate) max_value: u32,
    pub(crate) max_changes: usize,
}

impl Default for EntityCounterProbeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: 500,
            scan_bytes: 0x30000,
            max_value: 5000,
            max_changes: 48,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DamageFormulaProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) max_logs: usize,
}

impl Default for DamageFormulaProbeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_logs: 128,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RankRuntimeConfig {
    pub(crate) easy_s_rankable: bool,
    pub(crate) shift_count_thresholds: bool,
    pub(crate) shift_count_row_offset: Option<usize>,
    pub(crate) shift_count_rank_row_ids: Vec<u16>,
    pub(crate) shift_count_source_prefix: [u32; 3],
    pub(crate) shift_count_inserted_first: u32,
    pub(crate) shift_count_inserted_second: Option<u32>,
    pub(crate) count_threshold_override: Option<[u32; 5]>,
}

impl RankRuntimeConfig {
    pub(crate) const DEFAULT_SHIFT_COUNT_SOURCE_PREFIX: [u32; 3] = [60_000, 60_000, 48_000];
    pub(crate) const DEFAULT_SHIFT_COUNT_INSERTED_FIRST: u32 = 72_000;
}

impl Default for RankRuntimeConfig {
    fn default() -> Self {
        Self {
            easy_s_rankable: false,
            shift_count_thresholds: false,
            shift_count_row_offset: None,
            shift_count_rank_row_ids: Vec::new(),
            shift_count_source_prefix: Self::DEFAULT_SHIFT_COUNT_SOURCE_PREFIX,
            shift_count_inserted_first: Self::DEFAULT_SHIFT_COUNT_INSERTED_FIRST,
            shift_count_inserted_second: None,
            count_threshold_override: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpawnScalingProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) snapshot_interval_ms: u64,
    pub(crate) max_candidates: usize,
}

impl Default for SpawnScalingProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 1000,
            snapshot_interval_ms: 10000,
            max_candidates: 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FixedDataProbeConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) snapshot_interval_ms: u64,
}

impl Default for FixedDataProbeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 1000,
            snapshot_interval_ms: 5000,
        }
    }
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
pub(crate) struct RankHelperHooksConfig {
    pub(crate) enabled: bool,
    pub(crate) count_enabled: bool,
    pub(crate) merge_enabled: bool,
    pub(crate) callsite_enabled: bool,
    pub(crate) max_logs: usize,
}

impl Default for RankHelperHooksConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            count_enabled: false,
            merge_enabled: false,
            callsite_enabled: false,
            max_logs: 256,
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
