use std::sync::{Arc, Mutex};

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::{BridgeId, BridgeModContext, EventKey, HandlerDescriptor, HandlerRef, ModId};

use super::error;

pub(super) struct PendingHandlers(Arc<Mutex<PendingHandlerState>>);

impl PendingHandlers {
    pub(super) fn descriptors(self) -> Result<Vec<HandlerDescriptor>, String> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| "js handler registry lock poisoned".to_string())?;
        Ok(std::mem::take(&mut state.descriptors))
    }
}

struct PendingHandlerState {
    mod_id: ModId,
    bridge_id: BridgeId,
    next_id: usize,
    descriptors: Vec<HandlerDescriptor>,
}

pub(super) fn install(
    ctx: Ctx<'_>,
    context: &BridgeModContext,
) -> rquickjs::Result<PendingHandlers> {
    let state = pending_handlers(context);
    install_register_handler_ref(ctx, state.0.clone())?;
    Ok(state)
}

fn pending_handlers(context: &BridgeModContext) -> PendingHandlers {
    PendingHandlers(Arc::new(Mutex::new(PendingHandlerState {
        mod_id: context.mod_id.clone(),
        bridge_id: context.bridge_id.clone(),
        next_id: 0,
        descriptors: Vec::new(),
    })))
}

fn install_register_handler_ref(
    ctx: Ctx<'_>,
    callback_state: Arc<Mutex<PendingHandlerState>>,
) -> rquickjs::Result<()> {
    ctx.globals().set(
        "__oppw4_register_handler_ref",
        Func::from(move |event_key: String| -> rquickjs::Result<String> {
            register_handler_ref(&callback_state, event_key)
        }),
    )
}

fn register_handler_ref(
    callback_state: &Arc<Mutex<PendingHandlerState>>,
    event_key: String,
) -> rquickjs::Result<String> {
    let event_key = parse_event_key(event_key)?;
    let mut state = lock_pending_state(callback_state)?;
    let handler_ref = next_handler_ref(&mut state)?;
    push_descriptor(&mut state, event_key, handler_ref.clone());
    Ok(handler_ref.as_str().to_string())
}

fn parse_event_key(event_key: String) -> rquickjs::Result<EventKey> {
    EventKey::new(event_key)
        .map_err(|err| error::js_debug("String", "EventKey", "invalid event key", err))
}

fn lock_pending_state(
    callback_state: &Arc<Mutex<PendingHandlerState>>,
) -> rquickjs::Result<std::sync::MutexGuard<'_, PendingHandlerState>> {
    callback_state
        .lock()
        .map_err(|_| error::lock_poisoned("js handler registry"))
}

fn next_handler_ref(state: &mut PendingHandlerState) -> rquickjs::Result<HandlerRef> {
    state.next_id += 1;
    HandlerRef::new(format!("handler:{}", state.next_id))
        .map_err(|err| error::js_debug("String", "HandlerRef", "invalid handler ref", err))
}

fn push_descriptor(state: &mut PendingHandlerState, event_key: EventKey, handler_ref: HandlerRef) {
    let mod_id = state.mod_id.clone();
    let bridge_id = state.bridge_id.clone();
    state.descriptors.push(HandlerDescriptor {
        mod_id,
        bridge_id,
        event_key,
        handler_ref,
    });
}
