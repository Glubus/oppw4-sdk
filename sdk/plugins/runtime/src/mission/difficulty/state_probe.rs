mod labels;
mod snapshot;

use std::{
    thread,
    time::{Duration, Instant},
};

use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::{
    config::DifficultyProbeConfig,
    mission::difficulty::reward_row::{read_reward_row_dump, RewardRowDump},
    runtime::{
        core::difficulty::{
            update_snapshot as update_core_snapshot, DifficultyApplyEvent,
            DifficultyId as CoreDifficultyId, DifficultyMode as CoreDifficultyMode,
            DifficultySnapshot as CoreDifficultySnapshot,
        },
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
    publish_difficulty_event(host, snapshot);
}

fn publish_difficulty_event(host: &OwnedHostApi, snapshot: snapshot::DifficultySnapshot) {
    let event = DifficultyApplyEvent::new(core_snapshot_from_probe(snapshot));
    update_core_snapshot(event.snapshot.clone());
    let _ = host.log().write(PLUGIN_ID, difficulty_event_log(&event));
    signals::emit_json(
        host,
        signals::DIFFICULTY_EVENT,
        &DifficultyEventPayload::from(&event),
    );
}

fn core_snapshot_from_probe(snapshot: snapshot::DifficultySnapshot) -> CoreDifficultySnapshot {
    CoreDifficultySnapshot::new(
        CoreDifficultyMode::new(snapshot.mode_type.to_string()),
        CoreDifficultyId::new(snapshot.difficulty.to_string()),
    )
    .with_mission_id(u32::from(snapshot.mission_id))
}

fn difficulty_event_log(event: &DifficultyApplyEvent) -> String {
    let mission = event
        .snapshot
        .mission_id
        .map(|mission_id| mission_id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "difficulty_event mission={mission} mode={} difficulty={}",
        event.snapshot.mode.key(),
        event.snapshot.difficulty.key(),
    )
}

#[derive(Debug, Serialize)]
struct DifficultyEventPayload {
    schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    mission_id: Option<u32>,
    mode: String,
    difficulty: String,
}

impl From<&DifficultyApplyEvent> for DifficultyEventPayload {
    fn from(event: &DifficultyApplyEvent) -> Self {
        Self {
            schema: "sdk.runtime.difficulty.event.v1",
            mission_id: event.snapshot.mission_id,
            mode: event.snapshot.mode.key().to_string(),
            difficulty: event.snapshot.difficulty.key().to_string(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_snapshot_uses_probe_mode_and_difficulty() {
        let snapshot = snapshot::DifficultySnapshot {
            module_base: 0,
            global: 0,
            mission_id: 35,
            mode_type: 2,
            reward_mode: 0,
            difficulty: 3,
            special_flag: 0,
            cached_mission: 0,
            cached_difficulty: 0,
        };

        let core = core_snapshot_from_probe(snapshot);

        assert_eq!(core.mission_id, Some(35));
        assert_eq!(core.mode.key(), "treasure_log");
        assert_eq!(core.difficulty.key(), "super_hard");
    }

    #[test]
    fn difficulty_event_log_is_compact() {
        let event = DifficultyApplyEvent::new(
            CoreDifficultySnapshot::new(
                CoreDifficultyMode::new("treasure_log"),
                CoreDifficultyId::new("hard"),
            )
            .with_mission_id(77),
        );

        assert_eq!(
            difficulty_event_log(&event),
            "difficulty_event mission=77 mode=treasure_log difficulty=hard"
        );
    }
}
