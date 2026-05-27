use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use mlua::{Function, Lua, RegistryKey};
use sdk_bridge::{BridgeId, BridgeModContext, HandlerDescriptor, ModId};

pub(super) struct PendingHandlers(Arc<Mutex<PendingHandlerState>>);

impl PendingHandlers {
    pub(super) fn into_inner(
        self,
    ) -> Result<(BTreeMap<String, RegistryKey>, Vec<HandlerDescriptor>), String> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| "lua handler registry lock poisoned".to_string())?;
        Ok((
            std::mem::take(&mut state.handlers),
            std::mem::take(&mut state.descriptors),
        ))
    }
}

struct PendingHandlerState {
    mod_id: ModId,
    bridge_id: BridgeId,
    next_id: usize,
    handlers: BTreeMap<String, RegistryKey>,
    descriptors: Vec<HandlerDescriptor>,
}

pub(super) fn install(lua: &Lua, context: &BridgeModContext) -> mlua::Result<PendingHandlers> {
    let state = PendingHandlers(Arc::new(Mutex::new(PendingHandlerState {
        mod_id: context.mod_id.clone(),
        bridge_id: context.bridge_id.clone(),
        next_id: 0,
        handlers: BTreeMap::new(),
        descriptors: Vec::new(),
    })));
    let callback_state = state.0.clone();
    lua.globals().set(
        "__oppw4_register_handler",
        lua.create_function(move |lua, (event_key, callback): (String, Function)| {
            let event_key = sdk_bridge::EventKey::new(event_key)
                .map_err(|error| mlua::Error::external(format!("invalid event key: {error:?}")))?;
            let mut state = callback_state
                .lock()
                .map_err(|_| mlua::Error::external("lua handler registry lock poisoned"))?;
            state.next_id += 1;
            let handler_ref = sdk_bridge::HandlerRef::new(format!("handler:{}", state.next_id))
                .map_err(|error| {
                    mlua::Error::external(format!("invalid handler ref: {error:?}"))
                })?;
            let key = lua.create_registry_value(callback)?;
            state.handlers.insert(handler_ref.as_str().to_string(), key);
            let mod_id = state.mod_id.clone();
            let bridge_id = state.bridge_id.clone();
            state.descriptors.push(HandlerDescriptor {
                mod_id,
                bridge_id,
                event_key,
                handler_ref,
            });
            Ok(())
        })?,
    )?;
    Ok(state)
}
