mod snapshot;

use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::FixedDataProbeConfig,
    runtime::probe::{snapshot_interval, PLUGIN_ID},
};

pub(crate) fn start(host: OwnedHostApi, config: FixedDataProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "fixed_data_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_fixed_data_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: FixedDataProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "fixed_data_probe started interval_ms={} snapshot_interval_ms={}",
            interval.as_millis(),
            config.snapshot_interval_ms,
        ),
    );

    let snapshot_interval = snapshot_interval(config.snapshot_interval_ms);
    let mut last_snapshot_at = Instant::now()
        .checked_sub(snapshot_interval.unwrap_or(Duration::ZERO))
        .unwrap_or_else(Instant::now);
    let mut last_snapshot = None;
    let mut last_error = String::new();

    loop {
        thread::sleep(interval);
        match snapshot::read(&host) {
            Ok(snapshot) => {
                let periodic = snapshot_interval
                    .is_some_and(|interval| last_snapshot_at.elapsed() >= interval);
                if periodic || last_snapshot.as_ref() != Some(&snapshot) {
                    let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
                    last_snapshot = Some(snapshot);
                    last_snapshot_at = Instant::now();
                }
                last_error.clear();
            }
            Err(error) => {
                if error != last_error {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("fixed_data_probe pending: {error}"));
                    last_error = error;
                }
            }
        }
    }
}
