use std::{
    thread,
    time::{Duration, Instant},
};

use crate::runtime::fx::{
    config::{CycleConfig, CycleMode, FxConfig, TargetMode},
    log,
    mods::FxInstallPlan,
};

use super::{
    data::{write_f32, write_u32, write_usize, AuraDataPtrs},
    WorkerApi,
};

const ACTIVE_GATE_INTERVAL_MS: u64 = 250;
const ASSUMED_UPDATE_MS: f32 = 16.666_667;

pub(super) fn start_config_reloader(api: WorkerApi, data: AuraDataPtrs, initial: FxInstallPlan) {
    let interval = initial.plugin.refresh_interval_ms;
    if interval == 0 && !needs_runtime_loop(initial) {
        log::write_line("fx_director live config refresh disabled");
        return;
    }
    start_runtime_loop(api, data, initial, (interval != 0).then_some(interval));
}

fn start_runtime_loop(
    api: WorkerApi,
    data: AuraDataPtrs,
    initial: FxInstallPlan,
    config_reload_interval_ms: Option<u64>,
) {
    let _ = thread::Builder::new()
        .name("oppw4_fx_director_runtime".to_string())
        .spawn(move || {
            let started = Instant::now();
            let mut current = initial;
            let mut applied_fx = cycle_fx(current.fx, current.cycle, 0).unwrap_or(current.fx);
            let mut logged_definition = applied_fx;

            publish_active_character_to_shared_data(&api, data);
            apply_fx_data(data, applied_fx);
            let mut applied_enabled = effective_enabled(&api, applied_fx);
            write_u32(data.enabled, applied_enabled as u32);

            let mut last_active = format_active_character(&api);
            log_runtime_loop_start(current, config_reload_interval_ms, applied_enabled, &last_active);

            loop {
                thread::sleep(Duration::from_millis(loop_interval_ms(
                    current.cycle,
                    config_reload_interval_ms,
                )));

                let next = api.load_config().unwrap_or(current);
                let next_fx = cycle_fx(
                    next.fx,
                    next.cycle,
                    started.elapsed().as_millis() as u64,
                )
                .unwrap_or(next.fx);

                publish_active_character_to_shared_data(&api, data);
                let next_enabled = effective_enabled(&api, next_fx);
                if next_enabled != applied_enabled {
                    write_u32(data.enabled, next_enabled as u32);
                    log::write_line(format!(
                        "fx_director live active_gate enabled={} raw_enabled={} required_ids={} active={}",
                        next_enabled,
                        next_fx.enabled,
                        format_required_ids(next_fx),
                        format_active_character(&api)
                    ));
                    applied_enabled = next_enabled;
                }

                log_active_character_change(&api, next_fx, next_enabled, &mut last_active);
                log_cycle_change(current.cycle, next.cycle);
                if next_fx != applied_fx {
                    apply_fx_data(data, next_fx);
                    applied_fx = next_fx;
                }
                if next.fx != logged_definition {
                    log_fx_definition_change(logged_definition, next.fx);
                    logged_definition = next.fx;
                }
                current = next;
            }
        });
}

fn needs_runtime_loop(plan: FxInstallPlan) -> bool {
    plan.fx.required_character_id_count != 0
        || plan.fx.target == TargetMode::LocalPlayer
        || plan.cycle.is_active()
}

fn log_runtime_loop_start(
    plan: FxInstallPlan,
    config_reload_interval_ms: Option<u64>,
    applied_enabled: bool,
    active: &str,
) {
    if let Some(interval) = config_reload_interval_ms {
        log::write_line(format!(
            "fx_director live config refresh every {interval}ms active_gate every {interval}ms"
        ));
        return;
    }
    log::write_line(format!(
        "fx_director live config refresh disabled active_gate every {ACTIVE_GATE_INTERVAL_MS}ms required_ids={} initial_enabled={}",
        format_required_ids(plan.fx),
        applied_enabled
    ));
    log::write_line(format!(
        "fx_director active_gate initial required_ids={} active={} matched={}",
        format_required_ids(plan.fx),
        active,
        applied_enabled
    ));
}

fn loop_interval_ms(cycle: CycleConfig, config_reload_interval_ms: Option<u64>) -> u64 {
    let base = config_reload_interval_ms.unwrap_or(ACTIVE_GATE_INTERVAL_MS);
    if cycle.is_active() {
        match cycle.mode {
            CycleMode::FixedInterval => base.min(cycle.interval_ms).max(50),
            CycleMode::AfterAnimation => base.min(50),
        }
    } else {
        base
    }
}

fn cycle_fx(base: FxConfig, cycle: CycleConfig, elapsed_ms: u64) -> Option<FxConfig> {
    if !cycle.is_active() {
        return None;
    }
    let index = cycle_index(base, cycle, elapsed_ms)?;
    if let Some(preset) = cycle.preset_at(index) {
        return Some(preset);
    }
    cycle.effect_id_at(index).map(|effect_id| {
        let mut fx = base;
        fx.effect_id = effect_id;
        fx
    })
}

fn cycle_index(base: FxConfig, cycle: CycleConfig, elapsed_ms: u64) -> Option<usize> {
    let count = if cycle.preset_count > 1 {
        cycle.preset_count
    } else {
        cycle.effect_id_count
    } as usize;
    if count <= 1 {
        return None;
    }
    match cycle.mode {
        CycleMode::FixedInterval => {
            Some(((elapsed_ms / cycle.interval_ms) % count as u64) as usize)
        }
        CycleMode::AfterAnimation => animation_cycle_index(base, cycle, elapsed_ms, count),
    }
}

fn animation_cycle_index(
    base: FxConfig,
    cycle: CycleConfig,
    elapsed_ms: u64,
    count: usize,
) -> Option<usize> {
    let durations = (0..count)
        .map(|index| cycle.preset_at(index).unwrap_or(base))
        .map(animation_duration_ms)
        .collect::<Vec<_>>();
    let total = durations.iter().sum::<u64>();
    if total == 0 {
        return None;
    }
    let mut cursor = elapsed_ms % total;
    for (index, duration) in durations.iter().copied().enumerate() {
        if cursor < duration {
            return Some(index);
        }
        cursor -= duration;
    }
    Some(count - 1)
}

fn animation_duration_ms(fx: FxConfig) -> u64 {
    let step = fx.animation_speed.abs().max(0.001);
    let span = (fx.loop_end - fx.loop_start).abs().max(0.001);
    ((span / step) * ASSUMED_UPDATE_MS)
        .round()
        .clamp(50.0, 60_000.0) as u64
}

fn apply_fx_data(data: AuraDataPtrs, fx: FxConfig) {
    write_u32(data.effect_id, fx.effect_id);
    write_u32(data.force_effect_id, fx.force_effect_id as u32);
    write_u32(
        data.local_player_filter,
        (fx.target == TargetMode::LocalPlayer) as u32,
    );
    write_f32(data.speed, fx.animation_speed);
    write_f32(data.loop_start, fx.loop_start);
    write_f32(data.loop_end, fx.loop_end);
}

fn log_active_character_change(
    api: &WorkerApi,
    fx: FxConfig,
    enabled: bool,
    last_active: &mut String,
) {
    if fx.required_character_id_count == 0 && fx.target != TargetMode::LocalPlayer {
        return;
    }
    let active = format_active_character(api);
    if active == *last_active {
        return;
    }
    log::write_line(format!(
        "fx_director live active_character changed required_ids={} active={} matched={}",
        format_required_ids(fx),
        active,
        enabled
    ));
    *last_active = active;
}

fn log_cycle_change(current: CycleConfig, next: CycleConfig) {
    if current == next {
        return;
    }
    log::write_line(format!(
        "fx_director live cycle mode={:?} presets={} ids={} interval_ms={}",
        next.mode,
        next.preset_count,
        format_cycle_ids(next),
        next.interval_ms
    ));
}

fn log_fx_definition_change(current: FxConfig, next: FxConfig) {
    if next.effect_id != current.effect_id {
        log::write_line(format!(
            "fx_director live definition effect_id {} -> {}",
            current.effect_id, next.effect_id
        ));
    }
    if next.force_effect_id != current.force_effect_id {
        log::write_line(format!(
            "fx_director live definition force_effect_id {} -> {}",
            current.force_effect_id, next.force_effect_id
        ));
    }
    if next.animation_speed != current.animation_speed
        || next.loop_start != current.loop_start
        || next.loop_end != current.loop_end
    {
        log::write_line(format!(
            "fx_director live definition timing speed={} loop_start={} loop_end={}",
            next.animation_speed, next.loop_start, next.loop_end
        ));
    }
}

fn format_cycle_ids(cycle: CycleConfig) -> String {
    if cycle.effect_id_count == 0 {
        return "none".to_string();
    }
    cycle.effect_ids[..cycle.effect_id_count as usize]
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn effective_enabled(api: &WorkerApi, fx: FxConfig) -> bool {
    if !fx.enabled {
        return false;
    }
    let Some(active) = api.active_character() else {
        return fx.required_character_id_count == 0 && fx.target != TargetMode::LocalPlayer;
    };
    if fx.target == TargetMode::LocalPlayer && active.local_player == 0 {
        return false;
    }
    fx.accepts_active_character(active.runtime_id, active.alt_id)
}

fn publish_active_character_to_shared_data(api: &WorkerApi, data: AuraDataPtrs) {
    let Some(active) = api.active_character() else {
        return;
    };
    write_usize(data.local_player, active.local_player);
    write_usize(data.local_player_fx_owner, active.fx_owner);
}

fn format_required_ids(fx: FxConfig) -> String {
    if fx.required_character_id_count == 0 {
        return "any".to_string();
    }
    fx.required_character_ids[..fx.required_character_id_count as usize]
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_active_character(api: &WorkerApi) -> String {
    let Some(active) = api.active_character() else {
        return "unavailable".to_string();
    };
    format!(
        "runtime:{}({}) alt:{}({}) seq:{}",
        format_id(active.runtime_id),
        format_id_label(active.runtime_id),
        format_id(active.alt_id),
        format_id_label(active.alt_id),
        active.sequence
    )
}

fn format_id(id: u16) -> String {
    if id == u16::MAX {
        "none".to_string()
    } else {
        id.to_string()
    }
}

fn format_id_label(id: u16) -> String {
    if id == u16::MAX {
        return "none".to_string();
    }
    struct_api::find_by_id(id)
        .map(|character| character.canonical.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use crate::runtime::fx::config::{CycleConfig, CycleMode, FxConfig, TargetMode};

    use super::{animation_duration_ms, cycle_fx, loop_interval_ms};

    #[test]
    fn cycle_effect_id_changes_by_elapsed_interval() {
        let base = FxConfig::default();
        let cycle = CycleConfig {
            effect_ids: [10, 20, 0, 0, 0, 0, 0, 0],
            effect_id_count: 2,
            interval_ms: 500,
            ..CycleConfig::default()
        };

        assert_eq!(cycle_fx(base, cycle, 0).map(|fx| fx.effect_id), Some(10));
        assert_eq!(cycle_fx(base, cycle, 499).map(|fx| fx.effect_id), Some(10));
        assert_eq!(cycle_fx(base, cycle, 500).map(|fx| fx.effect_id), Some(20));
        assert_eq!(cycle_fx(base, cycle, 1000).map(|fx| fx.effect_id), Some(10));
    }

    #[test]
    fn cycle_presets_override_full_fx_definition() {
        let base = FxConfig::default();
        let first = FxConfig {
            effect_id: 10,
            target: TargetMode::LocalPlayer,
            ..FxConfig::default()
        };
        let second = FxConfig {
            effect_id: 20,
            animation_speed: 2.0,
            ..FxConfig::default()
        };
        let cycle = CycleConfig {
            presets: [
                first,
                second,
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
            ],
            preset_count: 2,
            interval_ms: 500,
            ..CycleConfig::default()
        };

        assert_eq!(cycle_fx(base, cycle, 0), Some(first));
        assert_eq!(cycle_fx(base, cycle, 500), Some(second));
    }

    #[test]
    fn cycle_keeps_reloader_awake_at_cycle_interval() {
        let cycle = CycleConfig {
            effect_ids: [10, 20, 0, 0, 0, 0, 0, 0],
            effect_id_count: 2,
            interval_ms: 75,
            ..CycleConfig::default()
        };

        assert_eq!(loop_interval_ms(cycle, None), 75);
    }

    #[test]
    fn animation_cycle_waits_for_each_preset_duration() {
        let base = FxConfig::default();
        let first = FxConfig {
            effect_id: 10,
            animation_speed: 0.016_666_667,
            loop_start: 0.1,
            loop_end: 1.9,
            ..FxConfig::default()
        };
        let second = FxConfig {
            effect_id: 20,
            ..first
        };
        let cycle = CycleConfig {
            mode: CycleMode::AfterAnimation,
            presets: [
                first,
                second,
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
                FxConfig::default(),
            ],
            preset_count: 2,
            ..CycleConfig::default()
        };

        assert_eq!(animation_duration_ms(first), 1800);
        assert_eq!(cycle_fx(base, cycle, 0).map(|fx| fx.effect_id), Some(10));
        assert_eq!(cycle_fx(base, cycle, 1799).map(|fx| fx.effect_id), Some(10));
        assert_eq!(cycle_fx(base, cycle, 1800).map(|fx| fx.effect_id), Some(20));
        assert_eq!(cycle_fx(base, cycle, 3600).map(|fx| fx.effect_id), Some(10));
    }
}
