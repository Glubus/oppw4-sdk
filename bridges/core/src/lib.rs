mod analysis;
mod context;
mod envelope;
mod error;
mod handler;
mod id;
mod manifest;
mod module;
mod record;
mod registry;
mod report;
mod traits;

pub use analysis::{
    analysis_warning, registry_declares_method, BridgeAnalysisReport, BridgeAnalysisWarning,
    BridgeModEffect, EffectConflict,
};
pub use context::{BridgeLoadRequest, BridgeModContext, BridgeModSource};
pub use envelope::{EventEnvelope, MutationEnvelope};
pub use error::BridgeError;
pub use handler::HandlerDescriptor;
pub use id::{BridgeId, EventKey, HandlerRef, ModId, MutationKey};
pub use manifest::{discover_mods, BridgeModManifest, BridgeModManifestError, DiscoveredBridgeMod};
pub use module::{
    RegistryEventDescriptor, RegistryFieldDescriptor, RegistryFunctionDescriptor,
    RegistryMethodDescriptor, RegistryModuleBuilder, RegistryModuleDescriptor, RegistryModuleLoad,
    RegistryModuleSchema, RegistryParamDescriptor, RegistrySchemaError, RegistryTypeDescriptor,
    RegistryTypeExtensionDescriptor, RegistryTypeRef,
};
pub use record::{ModLifecycle, ModRecord};
pub use registry::dispatch::HandlerConflict;
pub use registry::BridgeRegistry;
pub use report::{
    BridgeDispatchError, BridgeDispatchLog, BridgeDispatchReport, BridgeLoadReport,
    RegistryDispatchMetrics, RegistryDispatchReport,
};
pub use traits::RuntimeAdapter;

pub(crate) fn normalized_non_empty(
    value: String,
    field: &'static str,
) -> Result<String, BridgeError> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        Err(BridgeError::EmptyIdentifier { field })
    } else {
        Ok(value)
    }
}
