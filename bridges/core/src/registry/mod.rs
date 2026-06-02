use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    BridgeId, BridgeModEffect, EventKey, HandlerDescriptor, ModId, ModRecord, MutationEnvelope,
    RegistryModuleDescriptor, RuntimeAdapter,
};

pub mod dispatch;
mod load;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct BridgeRegistry {
    bridges: BTreeMap<BridgeId, SharedRuntimeAdapter>,
    modules: Vec<RegistryModuleDescriptor>,
    mods: BTreeMap<ModId, ModRecord>,
    handlers_by_event: BTreeMap<EventKey, Vec<HandlerDescriptor>>,
    effects_by_mod: BTreeMap<ModId, Vec<BridgeModEffect>>,
    boot_mutations: Vec<MutationEnvelope>,
    load_logs: Vec<String>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_runtime(&mut self, runtime: impl RuntimeAdapter + 'static) {
        self.bridges
            .insert(runtime.id(), Arc::new(Mutex::new(Box::new(runtime))));
    }

    pub fn register_module(&mut self, module: RegistryModuleDescriptor) {
        self.modules.retain(|known| {
            !(known.provider_id.eq_ignore_ascii_case(&module.provider_id)
                && known.module_name.eq_ignore_ascii_case(&module.module_name))
        });
        self.modules.push(module);
    }

    pub fn drain_boot_mutations(&mut self) -> Vec<MutationEnvelope> {
        std::mem::take(&mut self.boot_mutations)
    }

    pub fn drain_load_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.load_logs)
    }
}

pub(crate) type SharedRuntimeAdapter = Arc<Mutex<Box<dyn RuntimeAdapter>>>;
