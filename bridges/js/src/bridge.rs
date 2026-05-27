use std::collections::BTreeMap;

use sdk_bridge::{
    BridgeDispatchError, BridgeDispatchReport, BridgeError, BridgeId, BridgeLoadReport,
    BridgeLoadRequest, BridgeModContext, BridgeRegistry, EventEnvelope, HandlerDescriptor, ModId,
    RegistryModuleDescriptor, RuntimeAdapter,
};

use crate::{module::JsModule, vm};

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

    fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport {
        let modules = modules_for_context(&context.modules);
        match vm::load(&context, &modules) {
            Ok(vm) => {
                let handlers = vm.handler_descriptors().to_vec();
                let logs = vm.drain_logs();
                self.vms.insert(context.mod_id.clone(), vm);
                BridgeLoadReport {
                    handlers,
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

    fn unload_mod(&mut self, mod_id: &ModId) {
        self.vms.remove(mod_id);
    }
}

fn modules_for_context(descriptors: &[RegistryModuleDescriptor]) -> Vec<JsModule> {
    descriptors
        .iter()
        .filter_map(|descriptor| {
            Some(JsModule {
                plugin_id: descriptor.provider_id.clone(),
                module_name: descriptor.module_name.clone(),
                context: descriptor.module_context,
                register: descriptor.install?,
                load: descriptor.load,
                schema: descriptor.schema.clone(),
                invoke: descriptor.invoke.clone(),
            })
        })
        .collect()
}
