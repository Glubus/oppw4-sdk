pub(crate) const MAX_EFFECT_CYCLE_ITEMS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CycleMode {
    FixedInterval,
    AfterAnimation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CycleConfig {
    pub(crate) effect_ids: [u32; MAX_EFFECT_CYCLE_ITEMS],
    pub(crate) effect_id_count: u8,
    pub(crate) presets: [super::fx::FxConfig; MAX_EFFECT_CYCLE_ITEMS],
    pub(crate) preset_count: u8,
    pub(crate) mode: CycleMode,
    pub(crate) interval_ms: u64,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            effect_ids: [0; MAX_EFFECT_CYCLE_ITEMS],
            effect_id_count: 0,
            presets: [super::fx::FxConfig::default(); MAX_EFFECT_CYCLE_ITEMS],
            preset_count: 0,
            mode: CycleMode::FixedInterval,
            interval_ms: 1000,
        }
    }
}

impl CycleConfig {
    pub(crate) fn is_active(&self) -> bool {
        if self.effect_id_count <= 1 && self.preset_count <= 1 {
            return false;
        }
        self.mode == CycleMode::AfterAnimation || self.interval_ms != 0
    }

    pub(crate) fn effect_id_at(&self, index: usize) -> Option<u32> {
        (index < self.effect_id_count as usize).then_some(self.effect_ids[index])
    }

    pub(crate) fn preset_at(&self, index: usize) -> Option<super::fx::FxConfig> {
        (index < self.preset_count as usize).then_some(self.presets[index])
    }
}
