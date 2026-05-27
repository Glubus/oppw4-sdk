use crate::{
    BridgeDispatchError, EventEnvelope, EventKey, HandlerDescriptor, RegistryDispatchReport,
};

use super::BridgeRegistry;

impl BridgeRegistry {
    pub fn handlers_for(&self, event_key: &EventKey) -> &[HandlerDescriptor] {
        self.handlers_by_event
            .get(event_key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn dispatch_event(&mut self, event: &EventEnvelope) -> RegistryDispatchReport {
        let handlers = self.handlers_for(&event.key).to_vec();
        let mut report = RegistryDispatchReport::default();
        for handler in handlers {
            let Some(bridge) = self.bridges.get_mut(&handler.bridge_id) else {
                report.errors.push(BridgeDispatchError {
                    mod_id: handler.mod_id,
                    bridge_id: handler.bridge_id,
                    message: "runtime adapter is not registered".to_string(),
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
}
