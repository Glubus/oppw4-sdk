use crate::{BridgeError, BridgeId, HandlerDescriptor, ModId, MutationEnvelope};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BridgeLoadReport {
    pub handlers: Vec<HandlerDescriptor>,
    pub boot_mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub errors: Vec<BridgeError>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BridgeDispatchReport {
    pub mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub errors: Vec<BridgeDispatchError>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryDispatchReport {
    pub mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub errors: Vec<BridgeDispatchError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDispatchError {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub message: String,
}
