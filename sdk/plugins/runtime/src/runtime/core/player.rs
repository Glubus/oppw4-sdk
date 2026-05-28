use std::sync::{OnceLock, RwLock};

use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::runtime::signals;

pub(crate) const CHARACTER_CHANGED_EVENT: &str = "sdk.runtime.player.character_changed";

/// Stable modder-facing character id.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct CharacterId(String);

impl CharacterId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Active player context captured by runtime hooks.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PlayerSnapshot {
    pub(crate) active_character_ids: Vec<CharacterId>,
}

impl PlayerSnapshot {
    pub(crate) const fn new() -> Self {
        Self {
            active_character_ids: Vec::new(),
        }
    }

    pub(crate) fn with_active_character(mut self, character_id: impl Into<CharacterId>) -> Self {
        self.active_character_ids.push(character_id.into());
        self
    }
}

impl From<&str> for CharacterId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CharacterId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

static LATEST_PLAYER_SNAPSHOT: OnceLock<RwLock<PlayerSnapshot>> = OnceLock::new();
static EVENT_HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn initialize_events(host: OwnedHostApi) {
    let _ = EVENT_HOST.set(host);
}

pub(crate) fn latest_snapshot() -> PlayerSnapshot {
    latest_snapshot_store()
        .read()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

pub(crate) fn update_snapshot(snapshot: PlayerSnapshot) {
    let previous = latest_snapshot();
    if let Ok(mut latest) = latest_snapshot_store().write() {
        *latest = snapshot.clone();
    }
    if previous != snapshot {
        publish_character_changed(&snapshot);
    }
}

fn latest_snapshot_store() -> &'static RwLock<PlayerSnapshot> {
    LATEST_PLAYER_SNAPSHOT.get_or_init(|| RwLock::new(PlayerSnapshot::new()))
}

fn publish_character_changed(snapshot: &PlayerSnapshot) {
    let Some(host) = EVENT_HOST.get() else {
        return;
    };
    let Some(character_id) = snapshot
        .active_character_ids
        .first()
        .map(CharacterId::as_str)
    else {
        return;
    };
    let payload = CharacterChangedPayload {
        character_id,
        active_character_ids: snapshot
            .active_character_ids
            .iter()
            .map(CharacterId::as_str)
            .collect(),
    };
    signals::emit_json(host, CHARACTER_CHANGED_EVENT, &payload);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CharacterChangedPayload<'a> {
    character_id: &'a str,
    active_character_ids: Vec<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_snapshot_tracks_active_characters() {
        let snapshot = PlayerSnapshot::new()
            .with_active_character("luffy")
            .with_active_character("zoro");

        assert_eq!(
            snapshot.active_character_ids,
            [CharacterId::from("luffy"), CharacterId::from("zoro")]
        );
    }

    #[test]
    fn latest_snapshot_updates_source_of_truth() {
        let snapshot = PlayerSnapshot::new().with_active_character("runtime:1");

        update_snapshot(snapshot.clone());

        assert_eq!(latest_snapshot(), snapshot);
    }
}
