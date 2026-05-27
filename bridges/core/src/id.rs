use crate::{normalized_non_empty, BridgeError};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModId(String);

impl ModId {
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeError> {
        normalized_non_empty(value.into(), "mod id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BridgeId(String);

impl BridgeId {
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeError> {
        normalized_non_empty(value.into(), "bridge id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EventKey(String);

impl EventKey {
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeError> {
        normalized_non_empty(value.into(), "event key").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MutationKey(String);

impl MutationKey {
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeError> {
        normalized_non_empty(value.into(), "mutation key").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerRef {
    id: String,
}

impl HandlerRef {
    pub fn new(value: impl Into<String>) -> Result<Self, BridgeError> {
        normalized_non_empty(value.into(), "handler ref").map(|id| Self { id })
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }
}
