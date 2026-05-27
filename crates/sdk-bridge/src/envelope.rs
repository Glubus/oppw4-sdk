use crate::{EventKey, ModId, MutationKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope {
    pub key: EventKey,
    pub payload_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnvelope {
    pub key: MutationKey,
    pub source_mod: ModId,
    pub payload_json: String,
}
