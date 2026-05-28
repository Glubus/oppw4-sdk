use crate::{
    BridgeDispatchReport, BridgeId, BridgeLoadReport, BridgeLoadRequest, BridgeModContext,
    BridgeModSource, EventEnvelope, EventKey, HandlerDescriptor, HandlerRef, ModId, ModLifecycle,
    MutationEnvelope, MutationKey, RegistryModuleDescriptor, RegistryModuleLoad, RuntimeAdapter,
};

use super::*;

#[test]
fn load_mod_goes_through_registered_bridge() {
    let mut registry = BridgeRegistry::new();
    let mod_id = mod_id("example");
    registry.register_module(RegistryModuleDescriptor {
        provider_id: "core_api".to_string(),
        module_name: "core.api".to_string(),
        module_context: 0,
        install: None,
        invoke: None,
        load: RegistryModuleLoad::Always,
        schema: None,
    });
    registry.register_runtime(FakeBridge::new("fake"));

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
    registry.register_runtime(FakeBridge::new("fake"));
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
    assert_eq!(report.mod_logs.len(), 1);
    assert_eq!(report.mod_logs[0].mod_id.as_str(), "events");
    assert_eq!(report.mod_logs[0].message, "dispatch:on_tick");
}

struct FakeBridge {
    id: BridgeId,
}

impl FakeBridge {
    fn new(id: &str) -> Self {
        Self { id: bridge_id(id) }
    }
}

impl RuntimeAdapter for FakeBridge {
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
