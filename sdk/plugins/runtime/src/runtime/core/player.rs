use std::sync::{OnceLock, RwLock};

/// Stable modder-facing character id.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct CharacterId(String);

impl CharacterId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
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

pub(crate) fn latest_snapshot() -> PlayerSnapshot {
    latest_snapshot_store()
        .read()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

pub(crate) fn update_snapshot(snapshot: PlayerSnapshot) {
    if let Ok(mut latest) = latest_snapshot_store().write() {
        *latest = snapshot;
    }
}

fn latest_snapshot_store() -> &'static RwLock<PlayerSnapshot> {
    LATEST_PLAYER_SNAPSHOT.get_or_init(|| RwLock::new(PlayerSnapshot::new()))
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
