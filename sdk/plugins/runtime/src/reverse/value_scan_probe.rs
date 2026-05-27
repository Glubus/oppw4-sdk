mod format;
mod scan;
mod snapshot;

use std::{thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::{config::ValueProbeConfig, runtime::probe::PLUGIN_ID};

pub(crate) fn start(host: OwnedHostApi, config: ValueProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "value_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_value_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: ValueProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "value_probe started interval_ms={} scan_bytes=0x{:x} max_hits={} values={:?}",
            interval.as_millis(),
            config.scan_bytes,
            config.max_hits,
            config.values,
        ),
    );

    let mut last_snapshot = None;
    let mut last_error = String::new();
    loop {
        thread::sleep(interval);
        match snapshot::read(&host, &config) {
            Ok(snapshot) => {
                if last_snapshot.as_ref() != Some(&snapshot) {
                    log_snapshot(&host, &snapshot);
                    last_snapshot = Some(snapshot);
                }
                last_error.clear();
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("value_probe pending: {error}"));
                    last_error = error;
                }
            }
        }
    }
}

fn log_snapshot(host: &OwnedHostApi, snapshot: &snapshot::ValueSnapshot) {
    let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
}
