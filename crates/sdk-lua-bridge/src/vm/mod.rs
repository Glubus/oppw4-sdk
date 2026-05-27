mod handlers;
mod loader;
mod runtime;

pub use loader::{load, lua_mod_from_context};

use std::collections::BTreeMap;

use mlua::{Function, Lua, RegistryKey};
use sdk_bridge::{EventEnvelope, HandlerDescriptor};

pub struct LuaVm {
    lua: Lua,
    handlers: BTreeMap<String, RegistryKey>,
    handler_descriptors: Vec<HandlerDescriptor>,
}

impl LuaVm {
    pub(super) fn new(
        lua: Lua,
        handlers: BTreeMap<String, RegistryKey>,
        handler_descriptors: Vec<HandlerDescriptor>,
    ) -> Self {
        Self {
            lua,
            handlers,
            handler_descriptors,
        }
    }

    pub fn handler_descriptors(&self) -> &[HandlerDescriptor] {
        &self.handler_descriptors
    }

    pub fn dispatch(
        &self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> Result<(), String> {
        let Some(key) = self.handlers.get(handler.handler_ref.as_str()) else {
            return Err(format!(
                "lua handler {} is not registered",
                handler.handler_ref.as_str()
            ));
        };
        let callback = self
            .lua
            .registry_value::<Function>(key)
            .map_err(|error| format!("lua handler lookup failed: {error}"))?;
        callback
            .call::<()>((event.key.as_str().to_string(), event.payload_json.clone()))
            .map_err(|error| format!("lua handler call failed: {error}"))
    }
}
