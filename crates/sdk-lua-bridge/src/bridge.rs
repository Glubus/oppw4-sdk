use std::collections::BTreeMap;

use sdk_bridge::{
    BridgeDispatchError, BridgeDispatchReport, BridgeError, BridgeId, BridgeLoadReport,
    BridgeLoadRequest, BridgeModContext, BridgeModuleDescriptor, BridgeRegistry, EventEnvelope,
    HandlerDescriptor, LanguageBridge, ModId,
};

use crate::{module::LuaModule, vm};

pub struct LuaBridge {
    modules: Vec<LuaModule>,
    id: BridgeId,
    vms: BTreeMap<ModId, vm::LuaVm>,
}

impl LuaBridge {
    pub fn new(modules: Vec<LuaModule>) -> Self {
        Self::with_id("lua", modules).expect("static bridge id")
    }

    pub fn with_id(id: impl Into<String>, modules: Vec<LuaModule>) -> Result<Self, BridgeError> {
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
        registry.register_bridge(self);
    }

    fn module_descriptors(&self) -> Vec<BridgeModuleDescriptor> {
        let bridge_id = self.id.clone();
        self.modules
            .iter()
            .map(|module| BridgeModuleDescriptor {
                bridge_id: bridge_id.clone(),
                provider_id: module.plugin_id.clone(),
                module_name: module.module_name.clone(),
                load: module.load,
            })
            .collect()
    }
}

pub fn register_lua_bridge(registry: &mut BridgeRegistry, modules: Vec<LuaModule>) {
    LuaBridge::new(modules).register(registry);
}

impl LanguageBridge for LuaBridge {
    fn id(&self) -> BridgeId {
        self.id.clone()
    }

    fn supports(&self, request: &BridgeLoadRequest) -> bool {
        request
            .entry_file
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("lua"))
    }

    fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport {
        let modules = modules_for_context(&self.modules, &context.modules);
        let mod_entry = vm::lua_mod_from_context(&context);
        match vm::load(&context, &mod_entry, &modules) {
            Ok(vm) => {
                let handlers = vm.handler_descriptors().to_vec();
                self.vms.insert(context.mod_id.clone(), vm);
                BridgeLoadReport {
                    handlers,
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
                    message: "lua vm is not loaded".to_string(),
                }],
                ..BridgeDispatchReport::default()
            };
        };
        match vm.dispatch(handler, event) {
            Ok(()) => BridgeDispatchReport::default(),
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

fn modules_for_context(
    modules: &[LuaModule],
    descriptors: &[BridgeModuleDescriptor],
) -> Vec<LuaModule> {
    descriptors
        .iter()
        .filter_map(|descriptor| {
            modules.iter().find(|module| {
                module
                    .plugin_id
                    .eq_ignore_ascii_case(&descriptor.provider_id)
                    && module
                        .module_name
                        .eq_ignore_ascii_case(&descriptor.module_name)
            })
        })
        .cloned()
        .collect()
}
