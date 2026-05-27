use crate::{BridgeId, EventKey, HandlerRef, ModId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerDescriptor {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub event_key: EventKey,
    pub handler_ref: HandlerRef,
}
