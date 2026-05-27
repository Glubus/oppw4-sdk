mod format;
mod layout;
mod snapshot;

use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::RankThresholdProbeConfig,
    runtime::{
        probe::{snapshot_interval, PLUGIN_ID},
        signals,
    },
};

pub(crate) fn start(host: OwnedHostApi, config: RankThresholdProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_threshold_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(250));
    let _ = thread::Builder::new()
        .name("oppw4_rank_threshold_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: RankThresholdProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_threshold_probe started interval_ms={} snapshot_interval_ms={}",
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
    let mut last_error_at = Instant::now() - Duration::from_secs(60);

    loop {
        thread::sleep(interval);
        match snapshot::read(&host) {
            Ok(snapshot) => {
                last_error.clear();
                let changed = last_snapshot.as_ref() != Some(&snapshot);
                let periodic = snapshot_interval
                    .is_some_and(|interval| last_snapshot_at.elapsed() >= interval);
                if !changed && !periodic {
                    continue;
                }

                last_snapshot = Some(snapshot.clone());
                last_snapshot_at = Instant::now();
                let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
                signals::emit_json(&host, signals::RANK_SNAPSHOT, &snapshot.signal_payload());
            }
            Err(error) => {
                if error != last_error || last_error_at.elapsed() >= Duration::from_secs(10) {
                    let _ = host
                        .log()
                        .write(PLUGIN_ID, format!("rank_threshold_probe pending: {error}"));
                    last_error = error;
                    last_error_at = Instant::now();
                }
            }
        }
    }
}
