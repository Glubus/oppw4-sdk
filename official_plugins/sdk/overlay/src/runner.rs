use std::{
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};

use plugin_sdk::OwnedHostApi;

use crate::{backend::RendererProbe, config, config::OverlayConfig, panels, PLUGIN_ID};

pub(crate) fn start(host: OwnedHostApi, path: PathBuf) {
    let _ = thread::Builder::new()
        .name("oppw4_sdk_overlay".to_string())
        .spawn(move || run(host, path));
}

fn run(host: OwnedHostApi, path: PathBuf) {
    let mut state = OverlayState::default();
    let _ = host.log().write(
        PLUGIN_ID,
        format!("sdk_overlay watching config path={}", path.display()),
    );

    loop {
        reload_if_changed(&host, &path, &mut state);
        if state.config.enabled {
            probe_renderer(&host, &mut state);
            log_debug_panel_changes(&host, &mut state);
        }
        thread::sleep(Duration::from_millis(
            state.config.poll_interval_ms.max(250),
        ));
    }
}

fn log_debug_panel_changes(host: &OwnedHostApi, state: &mut OverlayState) {
    let Some(snapshot) = panels::debug_snapshot() else {
        return;
    };
    if state.last_debug_snapshot.as_ref() == Some(&snapshot) {
        return;
    }
    let _ = host.log().write(
        PLUGIN_ID,
        format!("debug_panel_snapshot bytes={}", snapshot.len()),
    );
    state.last_debug_snapshot = Some(snapshot);
}

fn reload_if_changed(host: &OwnedHostApi, path: &PathBuf, state: &mut OverlayState) {
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    if modified.is_some() && modified == state.loaded_at {
        return;
    }

    state.config = config::load(path);
    state.loaded_at = modified.or(Some(SystemTime::now()));
    state.last_probe = None;
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "sdk_overlay config loaded enabled={} backend={:?} poll_interval_ms={}",
            state.config.enabled, state.config.backend, state.config.poll_interval_ms,
        ),
    );
}

fn probe_renderer(host: &OwnedHostApi, state: &mut OverlayState) {
    let probe = RendererProbe::detect(state.config.backend);
    if !state.config.log_renderer_probe && state.last_probe.is_some() {
        return;
    }
    if state.last_probe.as_ref() == Some(&probe) {
        return;
    }
    let _ = host
        .log()
        .write(PLUGIN_ID, format!("renderer_probe {probe}"));
    state.last_probe = Some(probe);
}

#[derive(Default)]
struct OverlayState {
    config: OverlayConfig,
    loaded_at: Option<SystemTime>,
    last_probe: Option<RendererProbe>,
    last_debug_snapshot: Option<String>,
}
