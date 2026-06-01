use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::{BridgeId, BridgeModContext, EventKey, HandlerDescriptor, HandlerRef, ModId};

use super::error;

pub(super) struct PendingHandlers {
    descriptors: Receiver<HandlerDescriptor>,
    state: PendingHandlerState,
}

impl PendingHandlers {
    pub(super) fn descriptors(self) -> Result<Vec<HandlerDescriptor>, String> {
        Ok(self.descriptors.try_iter().collect())
    }
}

#[derive(Clone)]
struct PendingHandlerState {
    mod_id: ModId,
    bridge_id: BridgeId,
    next_id: Arc<AtomicUsize>,
    descriptors: Sender<HandlerDescriptor>,
}

pub(super) fn install(
    ctx: Ctx<'_>,
    context: &BridgeModContext,
) -> rquickjs::Result<PendingHandlers> {
    let state = pending_handlers(context);
    install_register_handler_ref(ctx, state.state.clone())?;
    Ok(state)
}

fn pending_handlers(context: &BridgeModContext) -> PendingHandlers {
    let (descriptors, receiver) = mpsc::channel();
    PendingHandlers {
        descriptors: receiver,
        state: PendingHandlerState {
            mod_id: context.mod_id.clone(),
            bridge_id: context.bridge_id.clone(),
            next_id: Arc::new(AtomicUsize::new(0)),
            descriptors,
        },
    }
}

fn install_register_handler_ref(
    ctx: Ctx<'_>,
    callback_state: PendingHandlerState,
) -> rquickjs::Result<()> {
    ctx.globals().set(
        "__oppw4_register_handler_ref",
        Func::from(move |event_key: String| -> rquickjs::Result<String> {
            let next_id = callback_state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
            register_handler_ref(&callback_state, next_id, event_key)
        }),
    )
}

fn register_handler_ref(
    callback_state: &PendingHandlerState,
    next_id: usize,
    event_key: String,
) -> rquickjs::Result<String> {
    let event_key = parse_event_key(event_key)?;
    let handler_ref = next_handler_ref(next_id)?;
    push_descriptor(callback_state, event_key, handler_ref.clone())?;
    Ok(handler_ref.as_str().to_string())
}

fn parse_event_key(event_key: String) -> rquickjs::Result<EventKey> {
    EventKey::new(event_key)
        .map_err(|err| error::js_debug("String", "EventKey", "invalid event key", err))
}

fn next_handler_ref(next_id: usize) -> rquickjs::Result<HandlerRef> {
    HandlerRef::new(format!("handler:{next_id}"))
        .map_err(|err| error::js_debug("String", "HandlerRef", "invalid handler ref", err))
}

fn push_descriptor(
    state: &PendingHandlerState,
    event_key: EventKey,
    handler_ref: HandlerRef,
) -> rquickjs::Result<()> {
    let mod_id = state.mod_id.clone();
    let bridge_id = state.bridge_id.clone();
    state
        .descriptors
        .send(HandlerDescriptor {
            mod_id,
            bridge_id,
            event_key,
            handler_ref,
        })
        .map_err(|_| error::js("Registry", "Handler", "handler receiver closed"))
}
