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
    pub(crate) fixed_data_probe: FixedDataProbeConfig,
    pub(crate) player_result_probe: PlayerResultProbeConfig,
    pub(crate) result_probe: ResultProbeConfig,
    pub(crate) rank_threshold_probe: RankThresholdProbeConfig,
    pub(crate) reward_probe: RewardProbeConfig,
    pub(crate) item_reward_probe: ItemRewardProbeConfig,
    pub(crate) result_state_probe: ResultStateProbeConfig,
    pub(crate) value_probe: ValueProbeConfig,
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
