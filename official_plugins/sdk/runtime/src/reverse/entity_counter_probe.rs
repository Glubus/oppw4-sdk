mod snapshot;

use std::{thread, time::Duration};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::EntityCounterProbeConfig,
    runtime::probe::{snapshot_interval, PLUGIN_ID},
};

pub(crate) fn start(host: OwnedHostApi, config: EntityCounterProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "entity_counter_probe disabled by config");
        return;
    }

    let _ = thread::Builder::new()
        .name("oppw4_entity_counter_probe".to_string())
        .spawn(move || run_loop(host, config));
}

fn run_loop(host: OwnedHostApi, config: EntityCounterProbeConfig) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "entity_counter_probe started interval_ms={} scan_bytes=0x{:x} max_value={} max_changes={}",
            config.interval_ms, config.scan_bytes, config.max_value, config.max_changes,
        ),
    );

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let mut last_error = String::new();
    let mut previous = None;

    loop {
        match snapshot::read(&host, &config, previous.as_ref()) {
            Ok(snapshot) => {
                if !snapshot.changes.is_empty() {
                    let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
                }
                previous = Some(snapshot.bytes);
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("entity_counter_probe pending: {error}"));
                    last_error = error;
                }
                previous = None;
            }
        }

        thread::sleep(snapshot_interval(config.interval_ms).unwrap_or(interval));
    }
}
