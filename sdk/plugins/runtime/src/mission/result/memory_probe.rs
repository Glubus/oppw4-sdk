mod diff;
mod snapshot;

use std::{thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::{config::ResultProbeConfig, runtime::probe::PLUGIN_ID};

pub(crate) fn start(host: OwnedHostApi, config: ResultProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "result_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_result_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: ResultProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "result_probe started interval_ms={} result_area_bytes={} max_changed_words={}",
            interval.as_millis(),
            config.result_area_bytes,
            config.max_changed_words,
        ),
    );

    let mut last_snapshot = None;
    let mut last_error = String::new();
    loop {
        thread::sleep(interval);
        match snapshot::read(&host, config.result_area_bytes) {
            Ok(snapshot) => {
                if last_snapshot.as_ref() != Some(&snapshot) {
                    log_snapshot(
                        &host,
                        last_snapshot.as_ref(),
                        &snapshot,
                        config.max_changed_words,
                    );
                    last_snapshot = Some(snapshot);
                }
                last_error.clear();
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("result_probe pending: {error}"));
                    last_error = error;
                }
            }
        }
    }
}

fn log_snapshot(
    host: &OwnedHostApi,
    previous: Option<&snapshot::ResultSnapshot>,
    snapshot: &snapshot::ResultSnapshot,
    max_changed_words: usize,
) {
    let changes = diff::describe(
        previous.map(|snapshot| &snapshot.area),
        &snapshot.area,
        max_changed_words,
    );
    let reason = previous.map_or("initial", |_| "changed");
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "result_probe {reason} mission_id={} difficulty={} mode_type={} reward_mode={} context=0x{:x} work_flag=0x{:x} area_hash=0x{:016x} global=0x{:x} words={changes}",
            snapshot.mission_id,
            snapshot.difficulty,
            snapshot.mode_type,
            snapshot.reward_mode,
            snapshot.context,
            snapshot.work_flag,
            snapshot.area_hash,
            snapshot.global,
        ),
    );
}
