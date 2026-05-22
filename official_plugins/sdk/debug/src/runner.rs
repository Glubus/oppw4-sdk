use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    format, memory,
    model::{DebugConfig, ScanHit, WatchValue},
    script, PLUGIN_ID,
};

pub(crate) fn start(host: OwnedHostApi, path: PathBuf) {
    let _ = thread::Builder::new()
        .name("oppw4_sdk_debug".to_string())
        .spawn(move || run(host, path));
}

fn run(host: OwnedHostApi, path: PathBuf) {
    let mut state = DebugState::default();
    let _ = host.log().write(
        PLUGIN_ID,
        format!("sdk_debug watching config path={}", path.display()),
    );

    loop {
        if let Some(config) = load_if_changed(&host, &path, &mut state) {
            state.config = config;
            state.last_watches.clear();
            state.last_scans.clear();
        }

        if state.config.enabled {
            tick(
                &host,
                &state.config,
                &mut state.last_watches,
                &mut state.last_scans,
            );
        }

        thread::sleep(Duration::from_millis(state.config.interval_ms.max(100)));
    }
}

fn load_if_changed(
    host: &OwnedHostApi,
    path: &PathBuf,
    state: &mut DebugState,
) -> Option<DebugConfig> {
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    if modified.is_some() && modified == state.loaded_at {
        return None;
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            log_reload_error(host, state, format!("read failed: {error}"));
            return None;
        }
    };
    match script::parse(&text) {
        Ok(config) => {
            state.loaded_at = modified.or(Some(SystemTime::now()));
            state.last_error.clear();
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "sdk_debug config loaded enabled={} watches={} scans={} interval_ms={}",
                    config.enabled,
                    config.watches.len(),
                    config.scans.len(),
                    config.interval_ms,
                ),
            );
            Some(config)
        }
        Err(error) => {
            log_reload_error(host, state, format!("parse failed: {error}"));
            None
        }
    }
}

fn tick(
    host: &OwnedHostApi,
    config: &DebugConfig,
    last_watches: &mut HashMap<String, WatchValue>,
    last_scans: &mut HashMap<String, Vec<ScanHit>>,
) {
    for watch in &config.watches {
        match memory::read_watch(host, watch) {
            Ok(value) => {
                if last_watches.get(&watch.id) != Some(&value) {
                    let _ = host.log().write(
                        PLUGIN_ID,
                        format!("watch {} {}", watch.id, format::watch_value(&value)),
                    );
                    last_watches.insert(watch.id.clone(), value);
                }
            }
            Err(error) => {
                let _ = host
                    .log()
                    .write(PLUGIN_ID, format!("watch {} pending: {error}", watch.id));
            }
        }
    }

    for scan in &config.scans {
        match memory::run_scan(host, scan) {
            Ok(hits) => {
                if last_scans.get(&scan.id) != Some(&hits) {
                    let _ = host.log().write(
                        PLUGIN_ID,
                        format!("scan {} hits={}", scan.id, format::scan_hits(&hits)),
                    );
                    last_scans.insert(scan.id.clone(), hits);
                }
            }
            Err(error) => {
                let _ = host
                    .log()
                    .write(PLUGIN_ID, format!("scan {} pending: {error}", scan.id));
            }
        }
    }
}

fn log_reload_error(host: &OwnedHostApi, state: &mut DebugState, error: String) {
    if state.last_error == error {
        return;
    }
    let _ = host
        .log()
        .write(PLUGIN_ID, format!("sdk_debug config {error}"));
    state.last_error = error;
}

#[derive(Default)]
struct DebugState {
    config: DebugConfig,
    loaded_at: Option<SystemTime>,
    last_error: String,
    last_watches: HashMap<String, WatchValue>,
    last_scans: HashMap<String, Vec<ScanHit>>,
}
