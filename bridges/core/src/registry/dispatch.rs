use std::collections::BTreeMap;

use crate::{
    BridgeDispatchError, BridgeId, BridgeModEffect, EffectConflict, EventEnvelope, EventKey,
    HandlerDescriptor, ModId, RegistryDispatchReport, RegistryQueryReport,
};

use super::{BridgeRegistry, SharedRuntimeAdapter};

impl BridgeRegistry {
    pub fn handlers_for(&self, event_key: &EventKey) -> &[HandlerDescriptor] {
        self.handlers_by_event
            .get(event_key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_handlers(&self, event_key: &EventKey) -> bool {
        !self.handlers_for(event_key).is_empty()
    }

    pub fn handler_conflicts(&self) -> Vec<HandlerConflict> {
        let mut conflicts = Vec::new();
        for (event_key, handlers) in &self.handlers_by_event {
            let mods = unique_handler_mods(handlers);
            if mods.len() > 1 {
                conflicts.push(HandlerConflict {
                    event_key: event_key.clone(),
                    mod_ids: mods,
                });
            }
        }
        conflicts
    }

    pub fn effect_conflicts(&self) -> Vec<EffectConflict> {
        let mut grouped: BTreeMap<String, (BridgeModEffect, Vec<ModId>)> = BTreeMap::new();
        for (mod_id, effects) in &self.effects_by_mod {
            for effect in effects {
                let key = effect.conflict_key();
                grouped
                    .entry(key)
                    .and_modify(|(_, mods)| {
                        if !mods.iter().any(|known| known == mod_id) {
                            mods.push(mod_id.clone());
                        }
                    })
                    .or_insert_with(|| (effect.clone(), vec![mod_id.clone()]));
            }
        }
        grouped
            .into_values()
            .filter_map(|(effect, mod_ids)| {
                (mod_ids.len() > 1).then_some(EffectConflict { effect, mod_ids })
            })
            .collect()
    }

    pub fn dispatch_plan(&self, event: &EventEnvelope) -> RegistryDispatchPlan {
        let handlers = self
            .handlers_by_event
            .get(&event.key)
            .cloned()
            .unwrap_or_default();
        let handler_count = handlers.len();
        let grouped_handlers = owned_handlers_by_bridge(handlers);
        let mut batches = Vec::new();
        let mut missing = Vec::new();
        for (bridge_id, bridge_handlers) in grouped_handlers {
            if let Some(bridge) = self.bridges.get(&bridge_id) {
                batches.push(RegistryDispatchBatch {
                    bridge_id,
                    bridge: bridge.clone(),
                    handlers: bridge_handlers,
                });
            } else {
                missing.extend(bridge_handlers);
            }
        }
        RegistryDispatchPlan {
            event: event.clone(),
            handler_count,
            batches,
            missing,
        }
    }

    pub fn dispatch_event(&self, event: &EventEnvelope) -> RegistryDispatchReport {
        self.dispatch_plan(event).execute()
    }

    pub fn query_plan(&self, event: &EventEnvelope) -> RegistryQueryPlan {
        let handlers = self
            .handlers_by_event
            .get(&event.key)
            .cloned()
            .unwrap_or_default();
        let handler_count = handlers.len();
        let handlers = handlers
            .into_iter()
            .map(|handler| {
                let bridge = self.bridges.get(&handler.bridge_id).cloned();
                RegistryQueryHandler { bridge, handler }
            })
            .collect();
        RegistryQueryPlan {
            event: event.clone(),
            handler_count,
            handlers,
        }
    }

    pub fn query_event(&self, event: &EventEnvelope) -> RegistryQueryReport {
        self.query_plan(event).execute()
    }
}

#[derive(Clone)]
pub struct RegistryDispatchPlan {
    event: EventEnvelope,
    handler_count: usize,
    batches: Vec<RegistryDispatchBatch>,
    missing: Vec<HandlerDescriptor>,
}

impl RegistryDispatchPlan {
    pub fn execute(self) -> RegistryDispatchReport {
        let started = std::time::Instant::now();
        let mut report = RegistryDispatchReport::default();
        report.metrics.payload_bytes = self.event.payload_json.len();
        report.metrics.handler_count = self.handler_count;

        for handler in self.missing {
            report.errors.push(BridgeDispatchError {
                mod_id: handler.mod_id.clone(),
                bridge_id: handler.bridge_id.clone(),
                message: "runtime adapter is not registered".to_string(),
            });
        }

        for batch in self.batches {
            report.metrics.bridge_batch_count += 1;
            match batch.bridge.lock() {
                Ok(mut bridge) => {
                    let handler_refs = batch.handlers.iter().collect::<Vec<_>>();
                    let bridge_report = bridge.dispatch_many(&handler_refs, &self.event);
                    report.metrics.vm_batch_count += bridge_report.vm_batch_count;
                    report.mutations.extend(bridge_report.mutations);
                    report.logs.extend(bridge_report.logs);
                    report.mod_logs.extend(bridge_report.mod_logs);
                    report.errors.extend(bridge_report.errors);
                }
                Err(_) => {
                    for handler in batch.handlers {
                        report.errors.push(BridgeDispatchError {
                            mod_id: handler.mod_id,
                            bridge_id: batch.bridge_id.clone(),
                            message: "runtime adapter lock is poisoned".to_string(),
                        });
                    }
                }
            };
        }
        report.metrics.dispatch_us = started.elapsed().as_micros();
        report
    }
}

#[derive(Clone)]
struct RegistryDispatchBatch {
    bridge_id: BridgeId,
    bridge: SharedRuntimeAdapter,
    handlers: Vec<HandlerDescriptor>,
}

#[derive(Clone)]
pub struct RegistryQueryPlan {
    event: EventEnvelope,
    handler_count: usize,
    handlers: Vec<RegistryQueryHandler>,
}

impl RegistryQueryPlan {
    pub fn execute(self) -> RegistryQueryReport {
        let started = std::time::Instant::now();
        let mut report = RegistryQueryReport::default();
        report.metrics.payload_bytes = self.event.payload_json.len();
        report.metrics.handler_count = self.handler_count;

        for query_handler in self.handlers {
            let handler = query_handler.handler;
            let Some(bridge) = query_handler.bridge else {
                report.errors.push(BridgeDispatchError {
                    mod_id: handler.mod_id.clone(),
                    bridge_id: handler.bridge_id.clone(),
                    message: "runtime adapter is not registered".to_string(),
                });
                continue;
            };
            report.metrics.bridge_batch_count += 1;
            match bridge.lock() {
                Ok(mut bridge) => {
                    let bridge_report = bridge.query(&handler, &self.event);
                    report.metrics.vm_batch_count += bridge_report.vm_batch_count.max(1);
                    report.logs.extend(bridge_report.logs.iter().cloned());
                    report.mod_logs.extend(bridge_report.mod_logs);
                    report.errors.extend(bridge_report.errors);
                    if bridge_report.result_json.is_some() {
                        report.result_json = bridge_report.result_json;
                        break;
                    }
                }
                Err(_) => report.errors.push(BridgeDispatchError {
                    mod_id: handler.mod_id,
                    bridge_id: handler.bridge_id,
                    message: "runtime adapter lock is poisoned".to_string(),
                }),
            };
        }
        report.metrics.dispatch_us = started.elapsed().as_micros();
        report
    }
}

#[derive(Clone)]
struct RegistryQueryHandler {
    bridge: Option<SharedRuntimeAdapter>,
    handler: HandlerDescriptor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerConflict {
    pub event_key: EventKey,
    pub mod_ids: Vec<ModId>,
}

fn unique_handler_mods(handlers: &[HandlerDescriptor]) -> Vec<ModId> {
    let mut seen = BTreeMap::<&ModId, ()>::new();
    let mut mods = Vec::new();
    for handler in handlers {
        if seen.insert(&handler.mod_id, ()).is_none() {
            mods.push(handler.mod_id.clone());
        }
    }
    mods
}

fn owned_handlers_by_bridge(
    handlers: Vec<HandlerDescriptor>,
) -> BTreeMap<BridgeId, Vec<HandlerDescriptor>> {
    let mut grouped: BTreeMap<BridgeId, Vec<HandlerDescriptor>> = BTreeMap::new();
    for handler in handlers {
        grouped
            .entry(handler.bridge_id.clone())
            .or_default()
            .push(handler);
    }
    grouped
}
