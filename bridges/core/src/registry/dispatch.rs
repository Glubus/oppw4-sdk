use std::collections::BTreeMap;

use crate::{
    BridgeDispatchError, BridgeId, BridgeModEffect, EffectConflict, EventEnvelope, EventKey,
    HandlerDescriptor, ModId, RegistryDispatchReport,
};

use super::BridgeRegistry;

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

    pub fn dispatch_event(&mut self, event: &EventEnvelope) -> RegistryDispatchReport {
        let started = std::time::Instant::now();
        let handlers = self
            .handlers_by_event
            .get(&event.key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let grouped_handlers = handlers_by_bridge(handlers);
        let mut report = RegistryDispatchReport::default();
        report.metrics.payload_bytes = event.payload_json.len();
        report.metrics.handler_count = handlers.len();

        for (bridge_id, bridge_handlers) in grouped_handlers {
            let Some(bridge) = self.bridges.get_mut(&bridge_id) else {
                for handler in bridge_handlers {
                    report.errors.push(BridgeDispatchError {
                        mod_id: handler.mod_id.clone(),
                        bridge_id: handler.bridge_id.clone(),
                        message: "runtime adapter is not registered".to_string(),
                    });
                }
                continue;
            };
            report.metrics.bridge_batch_count += 1;
            let bridge_report = bridge.dispatch_many(&bridge_handlers, event);
            report.metrics.vm_batch_count += bridge_report.vm_batch_count;
            report.mutations.extend(bridge_report.mutations);
            report.logs.extend(bridge_report.logs);
            report.mod_logs.extend(bridge_report.mod_logs);
            report.errors.extend(bridge_report.errors);
        }
        report.metrics.dispatch_us = started.elapsed().as_micros();
        report
    }
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

fn handlers_by_bridge(
    handlers: &[HandlerDescriptor],
) -> BTreeMap<BridgeId, Vec<&HandlerDescriptor>> {
    let mut grouped: BTreeMap<BridgeId, Vec<&HandlerDescriptor>> = BTreeMap::new();
    for handler in handlers {
        grouped
            .entry(handler.bridge_id.clone())
            .or_default()
            .push(handler);
    }
    grouped
}
