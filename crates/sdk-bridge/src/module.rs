use crate::BridgeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeModuleDescriptor {
    pub bridge_id: BridgeId,
    pub provider_id: String,
    pub module_name: String,
    pub load: BridgeModuleLoad,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BridgeModuleLoad {
    #[default]
    WhenPluginRequested,
    Always,
}
