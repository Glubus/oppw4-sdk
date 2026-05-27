#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ModId(String);

impl ModId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        normalized_non_empty(value.into(), "mod id").map(Self)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct BridgeId(String);

impl BridgeId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        normalized_non_empty(value.into(), "bridge id").map(Self)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct EventKey(String);

impl EventKey {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        normalized_non_empty(value.into(), "event key").map(Self)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct MutationKey(String);

impl MutationKey {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        normalized_non_empty(value.into(), "mutation key").map(Self)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HandlerRef {
    id: String,
}

impl HandlerRef {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, RegistryError> {
        normalized_non_empty(value.into(), "handler ref").map(|id| Self { id })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HandlerDescriptor {
    pub(crate) mod_id: ModId,
    pub(crate) bridge_id: BridgeId,
    pub(crate) event_key: EventKey,
    pub(crate) handler_ref: HandlerRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventEnvelope {
    pub(crate) key: EventKey,
    pub(crate) payload_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MutationEnvelope {
    pub(crate) key: MutationKey,
    pub(crate) source_mod: ModId,
    pub(crate) payload_json: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BridgeLoadReport {
    pub(crate) handlers: Vec<HandlerDescriptor>,
    pub(crate) boot_mutations: Vec<MutationEnvelope>,
    pub(crate) logs: Vec<String>,
    pub(crate) errors: Vec<RegistryError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModLifecycle {
    BootOnce,
    EventDriven,
}

impl ModLifecycle {
    fn infer(report: &BridgeLoadReport) -> Self {
        if report.handlers.is_empty() {
            Self::BootOnce
        } else {
            Self::EventDriven
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModRecord {
    pub(crate) mod_id: ModId,
    pub(crate) bridge_id: BridgeId,
    pub(crate) lifecycle: ModLifecycle,
}

#[derive(Default)]
pub(crate) struct SdkRegistry {
    bridges: BTreeMap<BridgeId, Box<dyn LanguageBridge>>,
    mods: BTreeMap<ModId, ModRecord>,
    handlers_by_event: BTreeMap<EventKey, Vec<HandlerDescriptor>>,
    boot_mutations: Vec<MutationEnvelope>,
}

impl SdkRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_bridge(&mut self, bridge: impl LanguageBridge + 'static) {
        self.bridges.insert(bridge.id(), Box::new(bridge));
    }

    pub(crate) fn register_loaded_mod(
        &mut self,
        mod_id: ModId,
        bridge_id: BridgeId,
        report: BridgeLoadReport,
    ) -> Result<ModLifecycle, RegistryError> {
        if !report.errors.is_empty() {
            return Err(RegistryError::LoadFailed {
                mod_id: mod_id.as_str().to_string(),
                errors: report.errors,
            });
        }

        let lifecycle = ModLifecycle::infer(&report);
        for handler in report.handlers {
            if handler.mod_id != mod_id {
                return Err(RegistryError::MismatchedModId {
                    expected: mod_id.as_str().to_string(),
                    actual: handler.mod_id.as_str().to_string(),
                });
            }
            if handler.bridge_id != bridge_id {
                return Err(RegistryError::MismatchedBridgeId {
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

    pub(crate) fn handlers_for(&self, event_key: &EventKey) -> &[HandlerDescriptor] {
        self.handlers_by_event
            .get(event_key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn drain_boot_mutations(&mut self) -> Vec<MutationEnvelope> {
        std::mem::take(&mut self.boot_mutations)
    }

    pub(crate) fn dispatch_event(&mut self, event: &EventEnvelope) -> RegistryDispatchReport {
        let handlers = self.handlers_for(&event.key).to_vec();
        let mut report = RegistryDispatchReport::default();
        for handler in handlers {
            let Some(bridge) = self.bridges.get_mut(&handler.bridge_id) else {
                report.errors.push(RegistryDispatchError {
                    mod_id: handler.mod_id,
                    bridge_id: handler.bridge_id,
                    message: "language bridge is not registered".to_string(),
                });
                continue;
            };
            let bridge_report = bridge.dispatch(&handler, event);
            report.mutations.extend(bridge_report.mutations);
            report.errors.extend(bridge_report.errors);
            report.logs.extend(bridge_report.logs);
        }
        report
    }

    #[cfg(test)]
    fn lifecycle_for(&self, mod_id: &ModId) -> Option<ModLifecycle> {
        self.mods.get(mod_id).map(|record| record.lifecycle)
    }
}

pub(crate) trait LanguageBridge: Send {
    fn id(&self) -> BridgeId;

    fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport;

    fn dispatch(
        &mut self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> BridgeDispatchReport;

    fn unload_mod(&mut self, mod_id: &ModId);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BridgeModContext {
    pub(crate) mod_id: ModId,
    pub(crate) bridge_id: BridgeId,
    pub(crate) name: String,
    pub(crate) source: BridgeModSource,
    pub(crate) entry_file: String,
    pub(crate) uses_plugins: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BridgeModSource {
    Directory(PathBuf),
    Zip { path: PathBuf, root: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BridgeDispatchReport {
    pub(crate) mutations: Vec<MutationEnvelope>,
    pub(crate) logs: Vec<String>,
    pub(crate) errors: Vec<RegistryDispatchError>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RegistryDispatchReport {
    pub(crate) mutations: Vec<MutationEnvelope>,
    pub(crate) logs: Vec<String>,
    pub(crate) errors: Vec<RegistryDispatchError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegistryDispatchError {
    pub(crate) mod_id: ModId,
    pub(crate) bridge_id: BridgeId,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RegistryError {
    EmptyIdentifier { field: &'static str },
    LoadFailed {
        mod_id: String,
        errors: Vec<RegistryError>,
    },
    MismatchedModId { expected: String, actual: String },
    MismatchedBridgeId { expected: String, actual: String },
    BridgeLoadError {
        mod_id: String,
        bridge_id: String,
        message: String,
    },
}

fn normalized_non_empty(value: String, field: &'static str) -> Result<String, RegistryError> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        Err(RegistryError::EmptyIdentifier { field })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_without_handlers_is_boot_once() {
        let mut registry = SdkRegistry::new();
        let mod_id = mod_id("ace_moveset");
        let bridge_id = bridge_id("lua");
        let mutation = mutation(&mod_id, "moveset.replace");
        let report = BridgeLoadReport {
            boot_mutations: vec![mutation.clone()],
            ..BridgeLoadReport::default()
        };

        let lifecycle = registry
            .register_loaded_mod(mod_id.clone(), bridge_id, report)
            .expect("register mod");

        assert_eq!(lifecycle, ModLifecycle::BootOnce);
        assert_eq!(registry.lifecycle_for(&mod_id), Some(ModLifecycle::BootOnce));
        assert_eq!(registry.drain_boot_mutations(), [mutation]);
    }

    #[test]
    fn mod_with_any_handler_is_event_driven() {
        let mut registry = SdkRegistry::new();
        let mod_id = mod_id("rank_logic");
        let bridge_id = bridge_id("lua");
        let event_key = event_key("sdk.runtime.rank.result");
        let handler = handler(&mod_id, &bridge_id, &event_key, "on_result:1");
        let report = BridgeLoadReport {
            handlers: vec![handler.clone()],
            ..BridgeLoadReport::default()
        };

        let lifecycle = registry
            .register_loaded_mod(mod_id.clone(), bridge_id, report)
            .expect("register mod");

        assert_eq!(lifecycle, ModLifecycle::EventDriven);
        assert_eq!(
            registry.handlers_for(&event_key),
            std::slice::from_ref(&handler)
        );
        assert_eq!(
            registry.lifecycle_for(&mod_id),
            Some(ModLifecycle::EventDriven)
        );
    }

    #[test]
    fn event_driven_mod_can_still_produce_boot_mutations() {
        let mut registry = SdkRegistry::new();
        let mod_id = mod_id("hybrid_mod");
        let bridge_id = bridge_id("lua");
        let event_key = event_key("sdk.runtime.reward.commit");
        let handler = handler(&mod_id, &bridge_id, &event_key, "on_commit:1");
        let mutation = mutation(&mod_id, "moveset.replace");
        let report = BridgeLoadReport {
            handlers: vec![handler],
            boot_mutations: vec![mutation.clone()],
            ..BridgeLoadReport::default()
        };

        let lifecycle = registry
            .register_loaded_mod(mod_id.clone(), bridge_id, report)
            .expect("register mod");

        assert_eq!(lifecycle, ModLifecycle::EventDriven);
        assert_eq!(registry.drain_boot_mutations(), [mutation]);
    }

    #[test]
    fn handler_must_match_loaded_mod_and_bridge() {
        let mut registry = SdkRegistry::new();
        let loaded_mod_id = mod_id("rank_logic");
        let bridge_id = bridge_id("lua");
        let bad_handler = handler(
            &mod_id("other_mod"),
            &bridge_id,
            &event_key("sdk.runtime.rank.result"),
            "on_result:1",
        );

        let error = registry
            .register_loaded_mod(
                loaded_mod_id,
                bridge_id,
                BridgeLoadReport {
                    handlers: vec![bad_handler],
                    ..BridgeLoadReport::default()
                },
            )
            .expect_err("mismatched handler should fail");

        assert!(matches!(error, RegistryError::MismatchedModId { .. }));
    }

    #[test]
    fn dispatch_event_calls_only_handlers_for_that_event_bridge() {
        let mut registry = SdkRegistry::new();
        let mod_id = mod_id("reward_logic");
        let bridge_id = bridge_id("fake");
        let reward_event_key = event_key("sdk.runtime.reward.commit");
        let rank_event_key = event_key("sdk.runtime.rank.result");
        registry.register_bridge(FakeBridge::new("fake"));
        registry
            .register_loaded_mod(
                mod_id.clone(),
                bridge_id.clone(),
                BridgeLoadReport {
                    handlers: vec![
                        handler(&mod_id, &bridge_id, &reward_event_key, "on_commit:1"),
                        handler(&mod_id, &bridge_id, &rank_event_key, "on_result:1"),
                    ],
                    ..BridgeLoadReport::default()
                },
            )
            .expect("register mod");

        let report = registry.dispatch_event(&EventEnvelope {
            key: reward_event_key,
            payload_json: "{\"berry\":100}".to_string(),
        });

        assert_eq!(report.errors, []);
        assert_eq!(report.logs, ["dispatch:on_commit:1"]);
        assert_eq!(report.mutations.len(), 1);
        assert_eq!(report.mutations[0].key, mutation_key("fake.mutation"));
    }

    #[test]
    fn dispatch_reports_missing_bridge_without_panicking() {
        let mut registry = SdkRegistry::new();
        let mod_id = mod_id("reward_logic");
        let bridge_id = bridge_id("missing");
        let event_key = event_key("sdk.runtime.reward.commit");
        registry
            .register_loaded_mod(
                mod_id.clone(),
                bridge_id.clone(),
                BridgeLoadReport {
                    handlers: vec![handler(&mod_id, &bridge_id, &event_key, "on_commit:1")],
                    ..BridgeLoadReport::default()
                },
            )
            .expect("register mod");

        let report = registry.dispatch_event(&EventEnvelope {
            key: event_key,
            payload_json: "{}".to_string(),
        });

        assert_eq!(report.mutations, []);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].mod_id, mod_id);
        assert_eq!(report.errors[0].bridge_id, bridge_id);
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

        fn load_mod(&mut self, _context: BridgeModContext) -> BridgeLoadReport {
            BridgeLoadReport::default()
        }

        fn dispatch(
            &mut self,
            handler: &HandlerDescriptor,
            _event: &EventEnvelope,
        ) -> BridgeDispatchReport {
            BridgeDispatchReport {
                mutations: vec![MutationEnvelope {
                    key: mutation_key("fake.mutation"),
                    source_mod: handler.mod_id.clone(),
                    payload_json: "{}".to_string(),
                }],
                logs: vec![format!("dispatch:{}", handler.handler_ref.as_str())],
                errors: Vec::new(),
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

    fn handler(
        mod_id: &ModId,
        bridge_id: &BridgeId,
        event_key: &EventKey,
        handler_ref: &str,
    ) -> HandlerDescriptor {
        HandlerDescriptor {
            mod_id: mod_id.clone(),
            bridge_id: bridge_id.clone(),
            event_key: event_key.clone(),
            handler_ref: HandlerRef::new(handler_ref).expect("handler ref"),
        }
    }

    fn mutation(mod_id: &ModId, key: &str) -> MutationEnvelope {
        MutationEnvelope {
            key: mutation_key(key),
            source_mod: mod_id.clone(),
            payload_json: "{}".to_string(),
        }
    }
}
