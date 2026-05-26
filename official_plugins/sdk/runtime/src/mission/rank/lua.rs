use std::sync::{Arc, Mutex};

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::runtime::core::{
    bus::RuntimeHandlerError,
    events::{RuntimeEvent, RuntimeMutation},
    live_bus,
    rank::{RankMutation, RankResultEvent, RankValue},
};

pub(super) const MODULE_NAME: &str = "sdk.runtime.ranks";

#[derive(Clone, Default)]
pub(super) struct RankLuaRegistry {
    callbacks: Arc<Mutex<Vec<RankLuaCallback>>>,
}

impl RankLuaRegistry {
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
    pub(super) fn dispatch_rank_result(
        &self,
        lua: &Lua,
        event: &RankResultEvent,
    ) -> RankLuaDispatchReport {
        let callbacks = match self.callbacks.lock() {
            Ok(callbacks) => callbacks,
            Err(error) => {
                return RankLuaDispatchReport {
                    mutations: Vec::new(),
                    errors: vec![error.to_string()],
                };
            }
        };
        let mut report = RankLuaDispatchReport::default();
        for callback in callbacks.iter() {
            let callback_fn = match lua.registry_value::<Function>(&callback.key) {
                Ok(callback_fn) => callback_fn,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            let mutations = Arc::new(Mutex::new(Vec::new()));
            let context = match rank_result_context(lua, event, Arc::clone(&mutations)) {
                Ok(context) => context,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            match callback_fn.call::<()>(context) {
                Ok(()) => match mutations.lock() {
                    Ok(mutations) => report.mutations.extend(mutations.iter().cloned()),
                    Err(error) => report.errors.push(error.to_string()),
                },
                Err(error) => report.errors.push(error.to_string()),
            }
        }
        report
    }
}

struct RankLuaCallback {
    #[cfg(test)]
    key: RegistryKey,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RankLuaDispatchReport {
    pub(super) mutations: Vec<RankMutation>,
    pub(super) errors: Vec<String>,
}

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_registry(lua, RankLuaRegistry::default())
}

pub(super) fn module_with_registry(lua: &Lua, registry: RankLuaRegistry) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("on_result", {
        let registry = registry.clone();
        lua.create_function(move |lua, callback: Function| {
            let mut callbacks = registry
                .callbacks
                .lock()
                .map_err(|_| mlua::Error::external("rank lua registry lock poisoned"))?;
            let callback_index = callbacks.len() + 1;
            #[cfg(test)]
            let key = lua.create_registry_value(callback.clone())?;
            callbacks.push(RankLuaCallback {
                #[cfg(test)]
                key,
            });
            register_live_rank_callback(lua, format!("lua:on_result:{callback_index}"), callback)?;
            Ok(())
        })?
    })?;
    Ok(table)
}

fn register_live_rank_callback(
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
    let live = Arc::new(Mutex::new(LiveRankCallback {
        id: handler_id.clone(),
        lua: Lua::clone(lua),
        key: lua.create_registry_value(callback)?,
    }));

    live_bus::register_runtime_handler(handler_id, move |event| {
        let RuntimeEvent::RankResult(event) = event else {
            return Ok(Vec::new());
        };
        let live = live
            .lock()
            .map_err(|_| RuntimeHandlerError::new("rank lua callback lock poisoned"))?;
        live.dispatch(event)
            .map(|mutations| mutations.into_iter().map(RuntimeMutation::Rank).collect())
            .map_err(|error| RuntimeHandlerError::new(error.to_string()))
    });
    Ok(())
}

struct LiveRankCallback {
    id: String,
    lua: Lua,
    key: RegistryKey,
}

impl LiveRankCallback {
    fn dispatch(&self, event: &RankResultEvent) -> mlua::Result<Vec<RankMutation>> {
        let callback_fn = self.lua.registry_value::<Function>(&self.key)?;
        let mutations = Arc::new(Mutex::new(Vec::new()));
        let context = rank_result_context(&self.lua, event, Arc::clone(&mutations))?;
        callback_fn
            .call::<()>(context)
            .map_err(|error| mlua::Error::external(format!("{} failed: {error}", self.id)))?;
        let mutations = mutations
            .lock()
            .map_err(|_| mlua::Error::external("rank lua mutation lock poisoned"))?;
        Ok(mutations.iter().cloned().collect())
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

fn rank_result_context(
    lua: &Lua,
    event: &RankResultEvent,
    mutations: Arc<Mutex<Vec<RankMutation>>>,
) -> mlua::Result<Table> {
    let context = lua.create_table()?;
    context.set("rank", runtime_rank(lua, event.rank, mutations)?)?;
    if let Some(mission_id) = event.mission_id {
        context.set("mission_id", mission_id)?;
    }
    Ok(context)
}

fn runtime_rank(
    lua: &Lua,
    rank: RankValue,
    mutations: Arc<Mutex<Vec<RankMutation>>>,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("slot", rank.slot())?;
    table.set("key", rank.key())?;
    table.set("alias", rank.debug_alias())?;
    table.set(
        "contains",
        lua.create_function(move |_lua, (_this, slots): (Table, Value)| {
            Ok(parse_slots(slots)?
                .iter()
                .any(|slot| rank_matches_normalized_slot(rank, slot)))
        })?,
    )?;
    table.set(
        "set_cap",
        lua.create_function(
            move |_, (_this, slot, enabled): (Table, Value, Option<bool>)| {
                for slot in parse_slots(slot)? {
                    mutations
                        .lock()
                        .map_err(|_| mlua::Error::external("rank lua mutation lock poisoned"))?
                        .push(RankMutation::SetCap {
                            rank: rank_value_from_normalized_slot(&slot),
                            enabled: enabled.unwrap_or(true),
                        });
                }
                Ok(())
            },
        )?,
    )?;
    Ok(table)
}

fn rank_matches_normalized_slot(rank: RankValue, slot: &str) -> bool {
    match rank {
        RankValue::D => slot == "d" || slot == "0",
        RankValue::C => slot == "c" || slot == "1",
        RankValue::B => slot == "b" || slot == "2",
        RankValue::A => slot == "a" || slot == "3",
        RankValue::S => slot == "s" || slot == "4",
        RankValue::SPlus => slot == "s_plus" || slot == "5",
        RankValue::Unknown(value) => slot == "unknown" || slot == value.to_string(),
    }
}

fn rank_value_from_normalized_slot(slot: &str) -> RankValue {
    match slot {
        "d" | "0" => RankValue::D,
        "c" | "1" => RankValue::C,
        "b" | "2" => RankValue::B,
        "a" | "3" => RankValue::A,
        "s" | "4" => RankValue::S,
        "s_plus" | "5" => RankValue::SPlus,
        _ => RankValue::Unknown(u8::MAX),
    }
}

fn parse_slots(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::Integer(slot) => Ok(vec![normalize_rank_slot(slot.to_string())]),
        Value::String(slot) => Ok(vec![normalize_rank_slot(slot.to_str()?.as_ref())]),
        Value::Table(table) => table.sequence_values::<Value>().map(parse_slot).collect(),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot or rank slot list".to_string(),
            message: None,
        }),
    }
}

fn parse_slot(value: mlua::Result<Value>) -> mlua::Result<String> {
    match value? {
        Value::Integer(slot) => Ok(normalize_rank_slot(slot.to_string())),
        Value::String(slot) => Ok(normalize_rank_slot(slot.to_str()?.as_ref())),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot".to_string(),
            message: None,
        }),
    }
}

fn normalize_rank_slot(value: impl AsRef<str>) -> String {
    match value
        .as_ref()
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .as_str()
    {
        "0" | "d" => "d".to_string(),
        "1" | "c" => "c".to_string(),
        "2" | "b" => "b".to_string(),
        "3" | "a" => "a".to_string(),
        "4" | "s" => "s".to_string(),
        "5" | "s+" | "s_plus" | "splus" => "s_plus".to_string(),
        slot => slot.to_string(),
    }
}
