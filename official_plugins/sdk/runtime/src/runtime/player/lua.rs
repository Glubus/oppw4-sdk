#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::runtime::core::{
    bus::RuntimeHandlerError,
    events::RuntimeEvent,
    live_bus,
    player::{CharacterId, PlayerChangeEvent},
};

pub(super) const MODULE_NAME: &str = "sdk.runtime.player";

#[derive(Clone, Default)]
pub(super) struct PlayerLuaRegistry {
    callbacks: Arc<Mutex<Vec<PlayerLuaCallback>>>,
}

impl PlayerLuaRegistry {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.callbacks
            .lock()
            .map(|callbacks| callbacks.len())
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(super) fn dispatch_player_change(
        &self,
        lua: &Lua,
        event: &PlayerChangeEvent,
    ) -> PlayerLuaDispatchReport {
        let callbacks = match self.callbacks.lock() {
            Ok(callbacks) => callbacks,
            Err(error) => {
                return PlayerLuaDispatchReport {
                    errors: vec![error.to_string()],
                };
            }
        };
        let mut report = PlayerLuaDispatchReport::default();
        for callback in callbacks.iter() {
            let callback_fn = match lua.registry_value::<Function>(&callback.key) {
                Ok(callback_fn) => callback_fn,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            let context = match player_change_context(lua, event) {
                Ok(context) => context,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            if let Err(error) = callback_fn.call::<()>(context) {
                report.errors.push(error.to_string());
            }
        }
        report
    }
}

struct PlayerLuaCallback {
    key: RegistryKey,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct PlayerLuaDispatchReport {
    pub(super) errors: Vec<String>,
}

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_registry(lua, PlayerLuaRegistry::default())
}

pub(super) fn module_with_registry(lua: &Lua, registry: PlayerLuaRegistry) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("on_change", {
        let registry = registry.clone();
        lua.create_function(move |lua, callback: Function| {
            let live_callback = callback.clone();
            let key = lua.create_registry_value(callback)?;
            let mut callbacks = registry
                .callbacks
                .lock()
                .map_err(|_| mlua::Error::external("player lua registry lock poisoned"))?;
            let callback_index = callbacks.len() + 1;
            callbacks.push(PlayerLuaCallback { key });
            register_live_player_callback(
                lua,
                format!("lua:on_change:{callback_index}"),
                live_callback,
            )?;
            Ok(())
        })?
    })?;
    Ok(table)
}

fn register_live_player_callback(
    lua: &Lua,
    callback_id: String,
    callback: Function,
) -> mlua::Result<()> {
    #[cfg(test)]
    if !live_callbacks_enabled(lua)? {
        return Ok(());
    }

    let mod_id = current_mod_id(lua)?;
    let handler_id = format!("{mod_id}:{callback_id}");
    let live = Arc::new(Mutex::new(LivePlayerCallback {
        id: handler_id.clone(),
        lua: Lua::clone(lua),
        key: lua.create_registry_value(callback)?,
    }));

    live_bus::register_runtime_handler(handler_id, move |event| {
        let RuntimeEvent::PlayerChange(event) = event else {
            return Ok(Vec::new());
        };
        let live = live
            .lock()
            .map_err(|_| RuntimeHandlerError::new("player lua callback lock poisoned"))?;
        live.dispatch(event)
            .map(|()| Vec::new())
            .map_err(|error| RuntimeHandlerError::new(error.to_string()))
    });
    Ok(())
}

struct LivePlayerCallback {
    id: String,
    lua: Lua,
    key: RegistryKey,
}

impl LivePlayerCallback {
    fn dispatch(&self, event: &PlayerChangeEvent) -> mlua::Result<()> {
        let callback_fn = self.lua.registry_value::<Function>(&self.key)?;
        let context = player_change_context(&self.lua, event)?;
        callback_fn
            .call::<()>(context)
            .map_err(|error| mlua::Error::external(format!("{} failed: {error}", self.id)))
    }
}

fn current_mod_id(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_id")
        .map(|id| id.unwrap_or_else(|| "unknown_mod".to_string()))
}

#[cfg(test)]
fn live_callbacks_enabled(lua: &Lua) -> mlua::Result<bool> {
    lua.globals()
        .get::<Option<bool>>("__oppw4_runtime_live_callbacks")
        .map(|enabled| enabled.unwrap_or(false))
}

fn player_change_context(lua: &Lua, event: &PlayerChangeEvent) -> mlua::Result<Table> {
    let context = lua.create_table()?;
    let active = lua.create_table()?;
    for (index, id) in event.snapshot.active_character_ids.iter().enumerate() {
        active.set(index + 1, id.as_str())?;
    }
    context.set("active_character_ids", active)?;
    context.set("has_active_character", {
        let ids = event.snapshot.active_character_ids.clone();
        lua.create_function(move |_, args: mlua::MultiValue| {
            let id = active_character_id(args)?.unwrap_or_default();
            Ok(has_character(&ids, &id))
        })?
    })?;
    Ok(context)
}

fn has_character(ids: &[CharacterId], id: &str) -> bool {
    ids.iter().any(|candidate| candidate.as_str() == id)
}

fn active_character(lua: &Lua, args: mlua::MultiValue) -> mlua::Result<Table> {
    match active_character_id(args)? {
        Some(id) => active_character_condition(lua, id),
        None => active_character_builder(lua),
    }
}

fn active_character_builder(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(
        "is",
        lua.create_function(|lua, (_this, id): (Table, String)| {
            active_character_condition(lua, id)
        })?,
    )?;
    Ok(table)
}

fn active_character_condition(lua: &Lua, id: String) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "active_character")?;
    table.set("id", id)?;
    Ok(table)
}

fn active_character_id(args: mlua::MultiValue) -> mlua::Result<Option<String>> {
    let mut values = args.into_iter();
    let first = values.next();
    let value = match first {
        Some(Value::Table(_)) => values.next(),
        other => other,
    };
    match value {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::String(id)) => Ok(Some(id.to_str()?.to_string())),
        Some(other) => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "character id".to_string(),
            message: None,
        }),
    }
}
