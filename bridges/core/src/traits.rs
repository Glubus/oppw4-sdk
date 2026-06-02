use crate::{
    BridgeDispatchLog, BridgeDispatchReport, BridgeId, BridgeLoadReport, BridgeLoadRequest,
    BridgeModContext, BridgeQueryReport, EventEnvelope, HandlerDescriptor, ModId,
};

pub trait RuntimeAdapter: Send {
    fn id(&self) -> BridgeId;

    fn supports(&self, request: &BridgeLoadRequest) -> bool;

    fn load_mod(&mut self, context: &BridgeModContext) -> BridgeLoadReport;

    fn dispatch(
        &mut self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> BridgeDispatchReport;

    fn dispatch_many(
        &mut self,
        handlers: &[&HandlerDescriptor],
        event: &EventEnvelope,
    ) -> BridgeDispatchReport {
        let mut report = BridgeDispatchReport::default();
        for handler in handlers {
            let handler_report = self.dispatch(handler, event);
            let handler_logs = handler_report.logs;
            report.mutations.extend(handler_report.mutations);
            report.logs.extend(handler_logs.iter().cloned());
            report
                .mod_logs
                .extend(handler_logs.into_iter().map(|message| BridgeDispatchLog {
                    mod_id: handler.mod_id.clone(),
                    bridge_id: handler.bridge_id.clone(),
                    message,
                }));
            report.errors.extend(handler_report.errors);
            report.vm_batch_count += handler_report.vm_batch_count.max(1);
        }
        report
    }

    fn query(&mut self, handler: &HandlerDescriptor, event: &EventEnvelope) -> BridgeQueryReport;

    fn unload_mod(&mut self, mod_id: &ModId);
}
