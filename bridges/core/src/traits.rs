use crate::{
    BridgeDispatchReport, BridgeId, BridgeLoadReport, BridgeLoadRequest, BridgeModContext,
    EventEnvelope, HandlerDescriptor, ModId,
};

pub trait RuntimeAdapter: Send {
    fn id(&self) -> BridgeId;

    fn supports(&self, request: &BridgeLoadRequest) -> bool;

    fn load_mod(&mut self, context: BridgeModContext) -> BridgeLoadReport;

    fn dispatch(
        &mut self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> BridgeDispatchReport;

    fn unload_mod(&mut self, mod_id: &ModId);
}
