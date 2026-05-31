use std::sync::Arc;

use crate::{EventKey, ModId, MutationKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope {
    pub key: EventKey,
    pub payload_json: Arc<str>,
}

impl EventEnvelope {
    pub fn new(key: EventKey, payload_json: impl Into<Arc<str>>) -> Self {
        Self {
            key,
            payload_json: payload_json.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnvelope {
    pub key: MutationKey,
    pub source_mod: ModId,
    pub payload_json: String,
}

impl MutationEnvelope {
    pub fn new(key: MutationKey, source_mod: ModId, payload_json: impl Into<String>) -> Self {
        Self {
            key,
            source_mod,
            payload_json: payload_json.into(),
        }
    }
}
