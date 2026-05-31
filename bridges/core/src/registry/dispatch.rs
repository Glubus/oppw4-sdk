use crate::{
    BridgeDispatchError, EffectConflict, EventEnvelope, EventKey, HandlerDescriptor,
    RegistryDispatchReport,
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
        let mut conflicts = Vec::new();
        let mut grouped: Vec<(String, crate::BridgeModEffect, Vec<crate::ModId>)> = Vec::new();
        for (mod_id, effects) in &self.effects_by_mod {
            for effect in effects {
                let key = effect.conflict_key();
                if let Some((_, _, mods)) = grouped
                    .iter_mut()
                    .find(|(known_key, _, _)| known_key == &key)
                {
                    if !mods.iter().any(|known| known == mod_id) {
                        mods.push(mod_id.clone());
                    }
                } else {
                    grouped.push((key, effect.clone(), vec![mod_id.clone()]));
                }
            }
        }
        for (_, effect, mod_ids) in grouped {
            if mod_ids.len() > 1 {
                conflicts.push(EffectConflict { effect, mod_ids });
            }
        }
        conflicts
    }

    pub fn dispatch_event(&mut self, event: &EventEnvelope) -> RegistryDispatchReport {
        let started = std::time::Instant::now();
        let handlers = self.handlers_for(&event.key).to_vec();
        let mut report = RegistryDispatchReport::default();
        report.metrics.payload_bytes = event.payload_json.len();
        report.metrics.handler_count = handlers.len();

        for (bridge_id, bridge_handlers) in handlers_by_bridge(handlers) {
            let Some(bridge) = self.bridges.get_mut(&bridge_id) else {
                for handler in bridge_handlers {
                    report.errors.push(BridgeDispatchError {
                        mod_id: handler.mod_id,
                        bridge_id: handler.bridge_id,
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
    pub mod_ids: Vec<crate::ModId>,
}

fn unique_handler_mods(handlers: &[HandlerDescriptor]) -> Vec<crate::ModId> {
    let mut mods = Vec::new();
    for handler in handlers {
        if !mods.iter().any(|known| known == &handler.mod_id) {
            mods.push(handler.mod_id.clone());
        }
    }
    mods
}

fn handlers_by_bridge(
    handlers: Vec<HandlerDescriptor>,
) -> Vec<(crate::BridgeId, Vec<HandlerDescriptor>)> {
    let mut grouped: Vec<(crate::BridgeId, Vec<HandlerDescriptor>)> = Vec::new();
    for handler in handlers {
        if let Some((_, existing)) = grouped
            .iter_mut()
            .find(|(bridge_id, _)| *bridge_id == handler.bridge_id)
        {
            existing.push(handler);
        } else {
            grouped.push((handler.bridge_id.clone(), vec![handler]));
        }
    }
    grouped
}
