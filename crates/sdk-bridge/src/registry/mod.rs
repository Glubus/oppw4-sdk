use std::collections::BTreeMap;

use crate::{
    BridgeDispatchError, BridgeError, BridgeId, BridgeLoadReport, BridgeLoadRequest,
    BridgeModContext, BridgeModuleDescriptor, BridgeModuleLoad, EventEnvelope, EventKey,
    HandlerDescriptor, LanguageBridge, ModId, ModLifecycle, ModRecord, MutationEnvelope,
    RegistryDispatchReport,
};

#[derive(Default)]
pub struct BridgeRegistry {
    bridges: BTreeMap<BridgeId, Box<dyn LanguageBridge>>,
    modules: Vec<BridgeModuleDescriptor>,
    mods: BTreeMap<ModId, ModRecord>,
    handlers_by_event: BTreeMap<EventKey, Vec<HandlerDescriptor>>,
    boot_mutations: Vec<MutationEnvelope>,
    load_logs: Vec<String>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_bridge(&mut self, bridge: impl LanguageBridge + 'static) {
        self.bridges.insert(bridge.id(), Box::new(bridge));
    }

    pub fn register_module(&mut self, module: BridgeModuleDescriptor) {
        self.modules.retain(|known| {
            !(known.bridge_id == module.bridge_id
                && known.provider_id.eq_ignore_ascii_case(&module.provider_id)
                && known.module_name.eq_ignore_ascii_case(&module.module_name))
        });
        self.modules.push(module);
    }

    pub fn load_supported_mod(
        &mut self,
        request: BridgeLoadRequest,
    ) -> Result<ModLifecycle, BridgeError> {
        let bridge_id = self.bridge_for(&request)?;
        let modules = self.modules_for(&bridge_id, &request.uses_plugins);
        self.load_mod(request.into_context(bridge_id, modules))
    }

    pub fn load_mod(&mut self, context: BridgeModContext) -> Result<ModLifecycle, BridgeError> {
        let mod_id = context.mod_id.clone();
        let bridge_id = context.bridge_id.clone();
        let Some(bridge) = self.bridges.get_mut(&bridge_id) else {
            return Err(BridgeError::MissingBridge {
                bridge_id: bridge_id.as_str().to_string(),
            });
        };
        let report = bridge.load_mod(context);
        self.register_loaded_mod(mod_id, bridge_id, report)
    }

    pub fn register_loaded_mod(
        &mut self,
        mod_id: ModId,
        bridge_id: BridgeId,
        report: BridgeLoadReport,
    ) -> Result<ModLifecycle, BridgeError> {
        if !report.errors.is_empty() {
            return Err(BridgeError::LoadFailed {
                mod_id: mod_id.as_str().to_string(),
                errors: report.errors,
            });
        }

        let lifecycle = ModLifecycle::infer(&report);
        self.load_logs.extend(report.logs);
        for handler in report.handlers {
            if handler.mod_id != mod_id {
                return Err(BridgeError::MismatchedModId {
                    expected: mod_id.as_str().to_string(),
                    actual: handler.mod_id.as_str().to_string(),
                });
            }
            if handler.bridge_id != bridge_id {
                return Err(BridgeError::MismatchedBridgeId {
                    expected: bridge_id.as_str().to_string(),
                    actual: handler.bridge_id.as_str().to_string(),
                });
            }
            self.handlers_by_event
                .entry(handler.event_key.clone())
                .or_default()
                .push(handler);
        }

        self.boot_mutations.extend(report.boot_mutations);
        self.mods.insert(
            mod_id.clone(),
            ModRecord {
                mod_id,
                bridge_id,
                lifecycle,
            },
        );
        Ok(lifecycle)
    }

    pub fn handlers_for(&self, event_key: &EventKey) -> &[HandlerDescriptor] {
        self.handlers_by_event
            .get(event_key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn drain_boot_mutations(&mut self) -> Vec<MutationEnvelope> {
        std::mem::take(&mut self.boot_mutations)
    }

    pub fn drain_load_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.load_logs)
    }

    pub fn dispatch_event(&mut self, event: &EventEnvelope) -> RegistryDispatchReport {
        let handlers = self.handlers_for(&event.key).to_vec();
        let mut report = RegistryDispatchReport::default();
        for handler in handlers {
            let Some(bridge) = self.bridges.get_mut(&handler.bridge_id) else {
                report.errors.push(BridgeDispatchError {
                    mod_id: handler.mod_id,
                    bridge_id: handler.bridge_id,
                    message: "language bridge is not registered".to_string(),
                });
                continue;
            };
            let bridge_report = bridge.dispatch(&handler, event);
            report.mutations.extend(bridge_report.mutations);
            report.logs.extend(bridge_report.logs);
            report.errors.extend(bridge_report.errors);
        }
        report
    }

    fn bridge_for(&self, request: &BridgeLoadRequest) -> Result<BridgeId, BridgeError> {
        let mut matches = self
            .bridges
            .iter()
            .filter(|(_, bridge)| bridge.supports(request))
            .map(|(id, _)| id.clone());
        let Some(first) = matches.next() else {
            return Err(BridgeError::NoBridgeForMod {
                mod_id: request.mod_id.as_str().to_string(),
                entry_file: request.entry_file.clone(),
            });
        };
        if matches.next().is_some() {
            return Err(BridgeError::AmbiguousBridgeForMod {
                mod_id: request.mod_id.as_str().to_string(),
                entry_file: request.entry_file.clone(),
            });
        }
        Ok(first)
    }

    fn modules_for(
        &self,
        bridge_id: &BridgeId,
        uses_plugins: &[String],
    ) -> Vec<BridgeModuleDescriptor> {
        self.modules
            .iter()
            .filter(|module| {
                module.bridge_id == *bridge_id
                    && (module.load == BridgeModuleLoad::Always
                        || uses_plugins
                            .iter()
                            .any(|plugin| module.provider_id.eq_ignore_ascii_case(plugin)))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        BridgeDispatchReport, BridgeLoadReport, BridgeLoadRequest, BridgeModContext,
        BridgeModSource, BridgeModuleDescriptor, BridgeModuleLoad, EventEnvelope, EventKey,
        HandlerDescriptor, HandlerRef, LanguageBridge, ModId, MutationEnvelope, MutationKey,
    };

    use super::*;

    #[test]
    fn load_mod_goes_through_registered_bridge() {
        let mut registry = BridgeRegistry::new();
        let mod_id = mod_id("example");
        let bridge_id = bridge_id("fake");
        registry.register_module(BridgeModuleDescriptor {
            bridge_id: bridge_id.clone(),
            provider_id: "core_api".to_string(),
            module_name: "core.api".to_string(),
            load: BridgeModuleLoad::Always,
        });
        registry.register_bridge(FakeBridge::new("fake"));

        let lifecycle = registry
            .load_supported_mod(BridgeLoadRequest {
                mod_id: mod_id.clone(),
                name: "Example".to_string(),
                source: BridgeModSource::Directory("example".into()),
                entry_file: "main.fake".to_string(),
                uses_plugins: Vec::new(),
            })
            .expect("load mod");

        assert_eq!(lifecycle, ModLifecycle::BootOnce);
        assert_eq!(registry.drain_boot_mutations()[0].source_mod, mod_id);
        assert_eq!(registry.drain_load_logs(), ["loaded"]);
    }

    #[test]
    fn dispatch_event_calls_registered_bridge_handlers() {
        let mut registry = BridgeRegistry::new();
        let mod_id = mod_id("events");
        let bridge_id = bridge_id("fake");
        let event_key = event_key("sdk.runtime.tick");
        registry.register_bridge(FakeBridge::new("fake"));
        registry
            .register_loaded_mod(
                mod_id.clone(),
                bridge_id.clone(),
                BridgeLoadReport {
                    handlers: vec![HandlerDescriptor {
                        mod_id,
                        bridge_id,
                        event_key: event_key.clone(),
                        handler_ref: HandlerRef::new("on_tick").expect("handler"),
                    }],
                    ..BridgeLoadReport::default()
                },
            )
            .expect("register mod");

        let report = registry.dispatch_event(&EventEnvelope {
            key: event_key,
            payload_json: "{}".to_string(),
        });

        assert_eq!(report.errors, []);
        assert_eq!(report.logs, ["dispatch:on_tick"]);
    }

    struct FakeBridge {
        id: BridgeId,
    }

    impl FakeBridge {
        fn new(id: &str) -> Self {
            Self { id: bridge_id(id) }
        }
    }

    impl LanguageBridge for FakeBridge {
        fn id(&self) -> BridgeId {
            self.id.clone()
        }

        fn supports(&self, request: &BridgeLoadRequest) -> bool {
            request.entry_file.ends_with(".fake")
        }

        fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport {
            assert_eq!(context.modules[0].module_name, "core.api");
            BridgeLoadReport {
                boot_mutations: vec![MutationEnvelope {
                    key: mutation_key("fake.boot"),
                    source_mod: context.mod_id,
                    payload_json: "{}".to_string(),
                }],
                logs: vec!["loaded".to_string()],
                ..BridgeLoadReport::default()
            }
        }

        fn dispatch(
            &mut self,
            handler: &HandlerDescriptor,
            _event: &EventEnvelope,
        ) -> BridgeDispatchReport {
            BridgeDispatchReport {
                logs: vec![format!("dispatch:{}", handler.handler_ref.as_str())],
                ..BridgeDispatchReport::default()
            }
        }

        fn unload_mod(&mut self, _mod_id: &ModId) {}
    }

    fn mod_id(value: &str) -> ModId {
        ModId::new(value).expect("mod id")
    }

    fn bridge_id(value: &str) -> BridgeId {
        BridgeId::new(value).expect("bridge id")
    }

    fn event_key(value: &str) -> EventKey {
        EventKey::new(value).expect("event key")
    }

    fn mutation_key(value: &str) -> MutationKey {
        MutationKey::new(value).expect("mutation key")
    }
}
