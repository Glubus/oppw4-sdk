use std::{sync::OnceLock, thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::runtime::fx::{
    config::{TargetMode, TriggerMode},
    log,
    mods::{FxInstallPlan, SharedFxState},
};

use super::WorkerApi;

static DEFERRED_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) fn install_deferred(api: OwnedHostApi, state: SharedFxState) -> i32 {
    if DEFERRED_STARTED.set(()).is_err() {
        log::write_line("fx_director deferred install already started");
        return 0;
    }
    let plugin = state.plugin_config();
    let delay = plugin.install_delay_ms;
    let worker_api = WorkerApi::new(api, state);
    let builder = thread::Builder::new().name("oppw4_fx_director".to_string());
    match builder.spawn(move || {
        log::write_line(format!("fx_director deferred install sleeping {delay}ms"));
        thread::sleep(Duration::from_millis(delay));
        if plugin.trigger == TriggerMode::Hotkey {
            log::write_line(format!(
                "fx_director waiting for hotkey vk=0x{:02x} before patching",
                plugin.hotkey_vk
            ));
            wait_for_hotkey(plugin.hotkey_vk);
            log::write_line("fx_director hotkey pressed, starting patch attempt");
        }
        if let Ok(status) = worker_api.api.game().status() {
            log::write_line(format!(
                "fx_director game_status before install phase={} flags=0x{:x} file_opens={} seconds={}",
                status.phase, status.flags, status.observed_file_opens, status.seconds_since_host_start
            ));
        }
        let config = wait_for_fx_plan(&worker_api);
        wait_for_active_character_gate(&worker_api, config);
        match worker_api.install_now(config) {
            Ok(()) => log::write_line("fx_director deferred install completed"),
            Err(error) => log::write_line(format!("fx_director deferred install failed: {error}")),
        }
    }) {
        Ok(_) => 0,
        Err(error) => {
            log::write_line(format!("fx_director deferred thread spawn failed: {error}"));
            -10
        }
    }
}

fn wait_for_fx_plan(api: &WorkerApi) -> FxInstallPlan {
    let mut tick = 0u32;
    loop {
        if let Some(config) = api.load_config() {
            log::write_line(format!(
                "fx_director fx plan ready effect_id={} target={:?} install_mode={:?}",
                config.fx.effect_id, config.fx.target, config.plugin.install_mode
            ));
            return config;
        }
        tick = tick.wrapping_add(1);
        if tick == 1 || tick.is_multiple_of(20) {
            log::write_line("fx_director waiting for fx definitions");
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn wait_for_active_character_gate(api: &WorkerApi, config: FxInstallPlan) {
    if !needs_active_character_before_install(config) {
        return;
    }
    log::write_line(format!(
        "fx_director waiting for active character before patching required_ids={} target={:?}",
        format_required_ids(config),
        config.fx.target
    ));
    let mut last_sequence = u64::MAX;
    let mut tick = 0u32;
    loop {
        if let Some(active) = api.active_character() {
            let has_local_player =
                config.fx.target != TargetMode::LocalPlayer || active.local_player != 0;
            let matches_character = config
                .fx
                .accepts_active_character(active.runtime_id, active.alt_id);
            if active.sequence != 0 && has_local_player && matches_character {
                log::write_line(format!(
                    "fx_director active character gate passed runtime={} alt={} local=0x{:x} owner=0x{:x} seq={}",
                    format_id(active.runtime_id),
                    format_id(active.alt_id),
                    active.local_player,
                    active.fx_owner,
                    active.sequence
                ));
                return;
            }
            if active.sequence != last_sequence {
                log::write_line(format!(
                    "fx_director active character gate pending runtime={} alt={} local=0x{:x} owner=0x{:x} seq={} matched={} local_ready={}",
                    format_id(active.runtime_id),
                    format_id(active.alt_id),
                    active.local_player,
                    active.fx_owner,
                    active.sequence,
                    matches_character,
                    has_local_player
                ));
                last_sequence = active.sequence;
            }
        }
        tick = tick.wrapping_add(1);
        if tick.is_multiple_of(20) {
            log::write_line("fx_director active character gate pending: no matching character yet");
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn needs_active_character_before_install(config: FxInstallPlan) -> bool {
    config.fx.target == TargetMode::LocalPlayer || config.fx.required_character_id_count != 0
}

fn format_required_ids(config: FxInstallPlan) -> String {
    if config.fx.required_character_id_count == 0 {
        return "any".to_string();
    }
    config.fx.required_character_ids[..config.fx.required_character_id_count as usize]
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_id(id: u16) -> String {
    if id == u16::MAX {
        "none".to_string()
    } else {
        id.to_string()
    }
}

fn wait_for_hotkey(vk: i32) {
    loop {
        let state = unsafe { GetAsyncKeyState(vk) };
        if state & 1 != 0 {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
}

#[cfg(test)]
mod tests {
    use crate::runtime::fx::{config::TargetMode, mods::FxInstallPlan};

    use super::needs_active_character_before_install;

    #[test]
    fn waits_for_local_player_target() {
        let mut config = FxInstallPlan::default();
        config.fx.target = TargetMode::LocalPlayer;

        assert!(needs_active_character_before_install(config));
    }

    #[test]
    fn waits_for_required_character_ids() {
        let mut config = FxInstallPlan::default();
        config.fx.target = TargetMode::All;
        config.fx.required_character_ids[0] = 26;
        config.fx.required_character_id_count = 1;

        assert!(needs_active_character_before_install(config));
    }

    #[test]
    fn global_effect_without_character_filter_can_install_immediately() {
        let mut config = FxInstallPlan::default();
        config.fx.target = TargetMode::All;

        assert!(!needs_active_character_before_install(config));
    }
}
