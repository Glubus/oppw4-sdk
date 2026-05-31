use crate::{
    BridgeAnalysisReport, BridgeError, BridgeId, HandlerDescriptor, ModId, MutationEnvelope,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BridgeLoadReport {
    pub handlers: Vec<HandlerDescriptor>,
    pub boot_mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub errors: Vec<BridgeError>,
    pub analysis: BridgeAnalysisReport,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BridgeDispatchReport {
    pub mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub errors: Vec<BridgeDispatchError>,
    pub vm_batch_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryDispatchReport {
    pub mutations: Vec<MutationEnvelope>,
    pub logs: Vec<String>,
    pub mod_logs: Vec<BridgeDispatchLog>,
    pub errors: Vec<BridgeDispatchError>,
    pub metrics: RegistryDispatchMetrics,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryDispatchMetrics {
    pub payload_bytes: usize,
    pub handler_count: usize,
    pub bridge_batch_count: usize,
    pub vm_batch_count: usize,
    pub dispatch_us: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDispatchLog {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDispatchError {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub message: String,
}
