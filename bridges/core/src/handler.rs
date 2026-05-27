use crate::{BridgeId, EventKey, HandlerRef, ModId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerDescriptor {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub event_key: EventKey,
    pub handler_ref: HandlerRef,
}

impl HandlerDescriptor {
    pub fn new(
        mod_id: ModId,
        bridge_id: BridgeId,
        event_key: EventKey,
        handler_ref: HandlerRef,
    ) -> Self {
        Self {
            mod_id,
            bridge_id,
            event_key,
            handler_ref,
        }
    }
}
