use std::path::PathBuf;

use crate::{BridgeId, BridgeModuleDescriptor, ModId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeLoadRequest {
    pub mod_id: ModId,
    pub name: String,
    pub source: BridgeModSource,
    pub entry_file: String,
    pub uses_plugins: Vec<String>,
}

impl BridgeLoadRequest {
    pub(crate) fn into_context(
        self,
        bridge_id: BridgeId,
        modules: Vec<BridgeModuleDescriptor>,
    ) -> BridgeModContext {
        BridgeModContext {
            mod_id: self.mod_id,
            bridge_id,
            name: self.name,
            source: self.source,
            entry_file: self.entry_file,
            uses_plugins: self.uses_plugins,
            modules,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeModContext {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub name: String,
    pub source: BridgeModSource,
    pub entry_file: String,
    pub uses_plugins: Vec<String>,
    pub modules: Vec<BridgeModuleDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeModSource {
    Directory(PathBuf),
    Zip { path: PathBuf, root: String },
}
