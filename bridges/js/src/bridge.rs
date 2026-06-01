use std::collections::BTreeMap;

use sdk_bridge::{
    BridgeDispatchError, BridgeDispatchLog, BridgeDispatchReport, BridgeError, BridgeId,
    BridgeLoadReport, BridgeLoadRequest, BridgeModContext, BridgeRegistry, EventEnvelope,
    HandlerDescriptor, ModId, RegistryModuleDescriptor, RuntimeAdapter,
};

use crate::{
    module::{JsModule, JsModuleRef},
    vm,
};

pub struct JsBridge {
    modules: Vec<JsModule>,
    id: BridgeId,
    vms: BTreeMap<ModId, vm::JsVm>,
}

impl JsBridge {
    pub fn new(modules: Vec<JsModule>) -> Self {
        Self::with_id("js", modules).expect("static bridge id")
    }

    pub fn with_id(id: impl Into<String>, modules: Vec<JsModule>) -> Result<Self, BridgeError> {
        Ok(Self {
            modules,
            id: BridgeId::new(id)?,
            vms: BTreeMap::new(),
        })
    }

    pub fn register(self, registry: &mut BridgeRegistry) {
        for module in self.module_descriptors() {
            registry.register_module(module);
        }
        registry.register_runtime(self);
    }

    fn module_descriptors(&self) -> Vec<RegistryModuleDescriptor> {
        self.modules
            .iter()
            .map(|module| {
                RegistryModuleDescriptor::builder(&module.plugin_id, &module.module_name)
                    .context(module.context)
                    .install(module.register)
                    .invoke_opt(module.invoke.clone())
                    .load(module.load)
                    .schema_opt(module.schema.clone())
                    .build()
            })
            .collect()
    }
}

pub fn register_js_bridge(registry: &mut BridgeRegistry, modules: Vec<JsModule>) {
    JsBridge::new(modules).register(registry);
}

impl RuntimeAdapter for JsBridge {
    fn id(&self) -> BridgeId {
        self.id.clone()
    }

    fn supports(&self, request: &BridgeLoadRequest) -> bool {
        request
            .entry_file
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("js"))
    }

    fn load_mod(&mut self, context: &BridgeModContext) -> BridgeLoadReport {
        let modules = module_refs_for_context(&context.modules);
        match vm::load(context, &modules) {
            Ok(vm) => {
                let handlers = vm.handler_descriptors().to_vec();
                let analysis = vm.analysis().clone();
                let logs = vm.drain_logs();
                self.vms.insert(context.mod_id.clone(), vm);
                BridgeLoadReport {
                    handlers,
                    analysis,
                    logs,
                    ..BridgeLoadReport::default()
                }
            }
            Err(error) => BridgeLoadReport {
                errors: vec![BridgeError::BridgeLoadError {
                    mod_id: context.mod_id.as_str().to_string(),
                    bridge_id: context.bridge_id.as_str().to_string(),
                    message: error,
                }],
                ..BridgeLoadReport::default()
            },
        }
    }

    fn dispatch(
        &mut self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> BridgeDispatchReport {
        let Some(vm) = self.vms.get(&handler.mod_id) else {
            return BridgeDispatchReport {
                errors: vec![BridgeDispatchError {
                    mod_id: handler.mod_id.clone(),
                    bridge_id: handler.bridge_id.clone(),
                    message: "js vm is not loaded".to_string(),
                }],
                ..BridgeDispatchReport::default()
            };
        };
        match vm.dispatch(handler, event) {
            Ok(()) => BridgeDispatchReport {
                logs: vm.drain_logs(),
                vm_batch_count: 1,
                ..BridgeDispatchReport::default()
            },
            Err(error) => BridgeDispatchReport {
                errors: vec![BridgeDispatchError {
                    mod_id: handler.mod_id.clone(),
                    bridge_id: handler.bridge_id.clone(),
                    message: error,
                }],
                ..BridgeDispatchReport::default()
            },
        }
    }

    fn dispatch_many(
        &mut self,
        handlers: &[&HandlerDescriptor],
        event: &EventEnvelope,
    ) -> BridgeDispatchReport {
        let mut report = BridgeDispatchReport::default();
        for (mod_id, mod_handlers) in handlers_by_mod(handlers) {
            let Some(vm) = self.vms.get(&mod_id) else {
                for handler in mod_handlers {
                    report.errors.push(BridgeDispatchError {
                        mod_id: handler.mod_id.clone(),
                        bridge_id: handler.bridge_id.clone(),
                        message: "js vm is not loaded".to_string(),
                    });
                }
                continue;
            };
            report.vm_batch_count += 1;
            match vm.dispatch_many(&mod_handlers, event) {
                Ok(()) => {
                    let logs = vm.drain_logs();
                    report.logs.extend(logs.iter().cloned());
                    report
                        .mod_logs
                        .extend(logs.into_iter().map(|message| BridgeDispatchLog {
                            mod_id: mod_id.clone(),
                            bridge_id: self.id.clone(),
                            message,
                        }));
                }
                Err(error) => {
                    for handler in mod_handlers {
                        report.errors.push(BridgeDispatchError {
                            mod_id: handler.mod_id.clone(),
                            bridge_id: handler.bridge_id.clone(),
                            message: error.clone(),
                        });
                    }
                }
            }
        }
        report
    }

    fn unload_mod(&mut self, mod_id: &ModId) {
        self.vms.remove(mod_id);
    }
}

fn handlers_by_mod<'a>(
    handlers: &[&'a HandlerDescriptor],
) -> BTreeMap<ModId, Vec<&'a HandlerDescriptor>> {
    let mut grouped: BTreeMap<ModId, Vec<&HandlerDescriptor>> = BTreeMap::new();
    for handler in handlers {
        grouped
            .entry(handler.mod_id.clone())
            .or_default()
            .push(handler);
    }
    grouped
}

fn module_refs_for_context(descriptors: &[RegistryModuleDescriptor]) -> Vec<JsModuleRef<'_>> {
    descriptors
        .iter()
        .filter_map(JsModuleRef::from_descriptor)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdk_bridge::RegistryModuleDescriptor;

    unsafe extern "system" fn install_stub(
        _module_context: *mut std::ffi::c_void,
        _js: *mut std::ffi::c_void,
    ) -> i32 {
        0
    }

    #[test]
    fn module_refs_for_context_borrows_registry_descriptors() {
        let descriptors = [RegistryModuleDescriptor::builder("sdk", "sdk.character")
            .install(install_stub)
            .build()];

        let modules = module_refs_for_context(&descriptors);

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].plugin_id, "sdk");
        assert_eq!(modules[0].module_name, "sdk.character");
    }
}
