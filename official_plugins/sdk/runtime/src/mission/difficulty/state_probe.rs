mod labels;
mod snapshot;

use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;

use crate::{
    config::DifficultyProbeConfig,
    mission::difficulty::reward_row::{read_reward_row_dump, RewardRowDump},
    runtime::{
        probe::{snapshot_interval, PLUGIN_ID},
        signals,
    },
};

pub(crate) fn start(host: OwnedHostApi, config: DifficultyProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "difficulty_probe disabled by config");
        return;
    }

    let interval = Duration::from_millis(config.interval_ms.max(50));
    let _ = thread::Builder::new()
        .name("oppw4_difficulty_probe".to_string())
        .spawn(move || run(host, config, interval));
}

fn run(host: OwnedHostApi, config: DifficultyProbeConfig, interval: Duration) {
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "difficulty_probe started interval_ms={} dump_reward_row={} snapshot_interval_ms={}",
            interval.as_millis(),
            config.dump_reward_row,
            config.snapshot_interval_ms,
        ),
    );

    let snapshot_interval = snapshot_interval(config.snapshot_interval_ms);
    let mut last_snapshot_at = Instant::now()
        .checked_sub(snapshot_interval.unwrap_or(Duration::ZERO))
        .unwrap_or_else(Instant::now);
    let mut last_snapshot = None;
    let mut last_reward_row = None;
    let mut last_error = String::new();
    let mut last_error_at = Instant::now() - Duration::from_secs(60);
    loop {
        thread::sleep(interval);
        match snapshot::read(&host) {
            Ok(snapshot) => handle_snapshot(
                &host,
                snapshot,
                &config,
                &mut last_snapshot,
                &mut last_snapshot_at,
                &mut last_reward_row,
            ),
            Err(error) => log_pending_error(&host, error, &mut last_error, &mut last_error_at),
        }
    }
}

fn handle_snapshot(
    host: &OwnedHostApi,
    snapshot: snapshot::DifficultySnapshot,
    config: &DifficultyProbeConfig,
    last_snapshot: &mut Option<snapshot::DifficultySnapshot>,
    last_snapshot_at: &mut Instant,
    last_reward_row: &mut Option<RewardRowDump>,
) {
    let interval = snapshot_interval(config.snapshot_interval_ms);
    let changed = *last_snapshot != Some(snapshot);
    let periodic = interval.is_some_and(|interval| last_snapshot_at.elapsed() >= interval);
    if !changed && !periodic {
        return;
    }

    *last_snapshot = Some(snapshot);
    *last_snapshot_at = Instant::now();
    log_snapshot(host, snapshot);
    if config.dump_reward_row {
        log_reward_row(host, snapshot, last_reward_row, periodic);
    }
}

fn log_pending_error(
    host: &OwnedHostApi,
    error: String,
    last_error: &mut String,
    last_error_at: &mut Instant,
) {
    if error != *last_error || last_error_at.elapsed() >= Duration::from_secs(10) {
        let _ = host
            .log()
            .write(PLUGIN_ID, format!("difficulty_probe pending: {error}"));
        *last_error = error;
        *last_error_at = Instant::now();
    }
}

fn log_snapshot(host: &OwnedHostApi, snapshot: snapshot::DifficultySnapshot) {
    let _ = host.log().write(PLUGIN_ID, snapshot.format_log());
    signals::emit_json(host, signals::DIFFICULTY_SNAPSHOT, &snapshot);
}

fn log_reward_row(
    host: &OwnedHostApi,
    snapshot: snapshot::DifficultySnapshot,
    last_reward_row: &mut Option<RewardRowDump>,
    force: bool,
) {
    match read_reward_row_dump(
        host,
        snapshot.module_base,
        snapshot.mission_id,
        snapshot.difficulty,
    ) {
        Ok(Some(row)) => {
            if !force && last_reward_row.as_ref() == Some(&row) {
                return;
            }
            let log = row.format_log();
            *last_reward_row = Some(row);
            let _ = host.log().write(PLUGIN_ID, log);
        }
        Ok(None) => log_missing_reward_row(host, snapshot, last_reward_row),
        Err(error) => {
            *last_reward_row = None;
            let _ = host
                .log()
                .write(PLUGIN_ID, format!("reward_row pending: {error}"));
        }
    }
}

fn log_missing_reward_row(
    host: &OwnedHostApi,
    snapshot: snapshot::DifficultySnapshot,
    last_reward_row: &mut Option<RewardRowDump>,
) {
    *last_reward_row = None;
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "reward_row unavailable mission_id={} difficulty={} reason=outside_base_reward_index",
            snapshot.mission_id, snapshot.difficulty
        ),
    );
}
