use std::sync::{Arc, Mutex};

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::{BridgeId, BridgeModContext, HandlerDescriptor, ModId};

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
    let state = PendingHandlers(Arc::new(Mutex::new(PendingHandlerState {
        mod_id: context.mod_id.clone(),
        bridge_id: context.bridge_id.clone(),
        next_id: 0,
        descriptors: Vec::new(),
    })));
    let callback_state = state.0.clone();
    ctx.globals().set(
        "__oppw4_register_handler_ref",
        Func::from(move |event_key: String| -> rquickjs::Result<String> {
            let event_key = sdk_bridge::EventKey::new(event_key).map_err(|error| {
                rquickjs::Error::new_from_js_message(
                    "String",
                    "EventKey",
                    format!("invalid event key: {error:?}"),
                )
            })?;
            let mut state = callback_state.lock().map_err(|_| {
                rquickjs::Error::new_from_js_message(
                    "Rust",
                    "Mutex",
                    "js handler registry lock poisoned",
                )
            })?;
            state.next_id += 1;
            let handler_ref = sdk_bridge::HandlerRef::new(format!("handler:{}", state.next_id))
                .map_err(|error| {
                    rquickjs::Error::new_from_js_message(
                        "String",
                        "HandlerRef",
                        format!("invalid handler ref: {error:?}"),
                    )
                })?;
            let mod_id = state.mod_id.clone();
            let bridge_id = state.bridge_id.clone();
            state.descriptors.push(HandlerDescriptor {
                mod_id,
                bridge_id,
                event_key,
                handler_ref: handler_ref.clone(),
            });
            Ok(handler_ref.as_str().to_string())
        }),
    )?;
    Ok(state)
}
