use std::sync::{Arc, Mutex};

use plugin_sdk::HostApi;

use crate::runtime::fx::{
    config::{load_plugin_config, CycleConfig, FxConfig, PluginConfig, TargetMode},
    log,
};

pub(crate) type SharedFxState = Arc<Mutex<FxRuntimeState>>;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FxInstallPlan {
    pub(crate) plugin: PluginConfig,
    pub(crate) fx: FxConfig,
    pub(crate) cycle: CycleConfig,
}

#[derive(Clone, Debug)]
pub(crate) struct FxRuntimeState {
    pub(crate) plugin: PluginConfig,
    pub(crate) effects: Vec<FxConfig>,
    effect_sources: Vec<String>,
    pub(crate) cycle: CycleConfig,
    cycle_source: Option<String>,
    pub(crate) current: Option<FxConfig>,
}

impl FxRuntimeState {
    pub(super) fn new(plugin: PluginConfig) -> Self {
        Self {
            plugin,
            effects: Vec::new(),
            effect_sources: Vec::new(),
            cycle: CycleConfig::default(),
            cycle_source: None,
            current: None,
        }
    }

    pub(crate) fn install_plan(&self) -> Option<FxInstallPlan> {
        self.current.map(|fx| FxInstallPlan {
            plugin: self.plugin,
            fx,
            cycle: self.cycle,
        })
    }

    pub(crate) fn plugin_config(&self) -> PluginConfig {
        self.plugin
    }

    pub(super) fn clear_mod_source(&mut self, source: &str) {
        let before = self.effects.len();
        let mut kept_effects = Vec::with_capacity(self.effects.len());
        let mut kept_sources = Vec::with_capacity(self.effect_sources.len());
        for (fx, owner) in self.effects.iter().copied().zip(self.effect_sources.iter()) {
            if !owner.eq_ignore_ascii_case(source) {
                kept_effects.push(fx);
                kept_sources.push(owner.clone());
            }
        }
        self.effects = kept_effects;
        self.effect_sources = kept_sources;
        if self
            .cycle_source
            .as_ref()
            .is_some_and(|owner| owner.eq_ignore_ascii_case(source))
        {
            self.cycle = CycleConfig::default();
            self.cycle_source = None;
        }
        self.current = self
            .cycle
            .preset_at(0)
            .or_else(|| self.effects.last().copied());
        let removed = before.saturating_sub(self.effects.len());
        if removed != 0 {
            log::write_line(format!(
                "fx_director hot reload: cleared {removed} fx definition(s) from mod={source}"
            ));
        }
    }

    pub(super) fn push_effect(&mut self, source: &str, fx: FxConfig) -> usize {
        self.warn_if_overlapping_effect(fx);
        self.current = Some(fx);
        let index = self.effects.len();
        self.effects.push(fx);
        self.effect_sources.push(source.to_string());
        index
    }

    pub(super) fn update_effect(&mut self, index: usize, update: impl FnOnce(&mut FxConfig)) {
        if let Some(fx) = self.effects.get_mut(index) {
            update(fx);
            self.current = Some(*fx);
        }
    }

    pub(super) fn set_cycle_presets(
        &mut self,
        presets: Vec<FxConfig>,
        source: &str,
        options: super::fx_module::CycleOptions,
    ) {
        if self.cycle.is_active()
            && self
                .cycle_source
                .as_ref()
                .is_some_and(|owner| !owner.eq_ignore_ascii_case(source))
        {
            log::write_line(format!(
                "fx_director warning: replacing fx cycle from mod={} with mod={source}; latest Lua mod wins",
                self.cycle_source.as_deref().unwrap_or("unknown")
            ));
        }
        self.cycle.set_presets(presets);
        self.cycle_source = Some(source.to_string());
        if let Some(mode) = options.mode {
            self.cycle.mode = mode;
        }
        if let Some(interval_ms) = options.interval_ms {
            self.cycle.interval_ms = interval_ms;
        }
        if let Some(first) = self.cycle.preset_at(0) {
            self.current = Some(first);
        }
        log::write_line(format!(
            "fx_director mod={} cycle updated mode={:?} presets={} first={}",
            source,
            self.cycle.mode,
            self.cycle.preset_count,
            self.cycle
                .preset_at(0)
                .map(format_fx_summary)
                .unwrap_or_else(|| "none".to_string())
        ));
    }

    fn warn_if_overlapping_effect(&self, next: FxConfig) {
        for existing in &self.effects {
            if fx_targets_overlap(*existing, next) {
                log::write_line(format!(
                    "fx_director warning: multiple fx definitions target the same character scope existing={} next={}; latest definition may override unless a cycle uses both",
                    format_fx_scope(*existing),
                    format_fx_scope(next)
                ));
                return;
            }
        }
    }
}

fn fx_targets_overlap(left: FxConfig, right: FxConfig) -> bool {
    if left.target == TargetMode::LocalPlayer || right.target == TargetMode::LocalPlayer {
        return left.target == right.target;
    }
    if left.required_character_id_count == 0 || right.required_character_id_count == 0 {
        return true;
    }
    left.required_character_ids[..left.required_character_id_count as usize]
        .iter()
        .any(|id| {
            right.required_character_ids[..right.required_character_id_count as usize].contains(id)
        })
}

fn format_fx_scope(fx: FxConfig) -> String {
    if fx.target == TargetMode::LocalPlayer {
        return "local_player".to_string();
    }
    if fx.required_character_id_count == 0 {
        return "all".to_string();
    }
    fx.required_character_ids[..fx.required_character_id_count as usize]
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_fx_summary(fx: FxConfig) -> String {
    format!(
        "effect_id={} target={:?} speed={} loop_start={} loop_end={} required={}",
        fx.effect_id,
        fx.target,
        fx.animation_speed,
        fx.loop_start,
        fx.loop_end,
        format_fx_scope(fx)
    )
}

pub(crate) fn load_config(host: HostApi<'_>) -> SharedFxState {
    Arc::new(Mutex::new(FxRuntimeState::new(load_plugin_config(host))))
}
