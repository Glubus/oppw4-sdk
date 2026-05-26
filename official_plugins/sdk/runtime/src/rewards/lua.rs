use std::sync::{Arc, Mutex};

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::runtime::core::{
    bus::RuntimeHandlerError,
    events::{RuntimeEvent, RuntimeMutation},
    live_bus,
    rank::RankValue,
    rewards::{RewardCommitEvent, RewardMutation},
};

pub(super) const MODULE_NAME: &str = "sdk.runtime.rewards";

#[derive(Clone, Default)]
pub(super) struct RewardLuaRegistry {
    callbacks: Arc<Mutex<Vec<RewardLuaCallback>>>,
}

impl RewardLuaRegistry {
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
    pub(super) fn dispatch_reward_commit(
        &self,
        lua: &Lua,
        event: &RewardCommitEvent,
    ) -> RewardLuaDispatchReport {
        let callbacks = match self.callbacks.lock() {
            Ok(callbacks) => callbacks,
            Err(error) => {
                return RewardLuaDispatchReport {
                    mutations: Vec::new(),
                    errors: vec![RewardLuaDispatchError {
                        callback_id: "registry".to_string(),
                        message: error.to_string(),
                    }],
                };
            }
        };
        let mut report = RewardLuaDispatchReport::default();

        for callback in callbacks.iter() {
            let callback_fn = match lua.registry_value::<Function>(&callback.key) {
                Ok(callback_fn) => callback_fn,
                Err(error) => {
                    report.errors.push(RewardLuaDispatchError {
                        callback_id: callback.id.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };

            let mutations = Arc::new(Mutex::new(Vec::new()));
            let context = match reward_commit_context(lua, event, Arc::clone(&mutations)) {
                Ok(context) => context,
                Err(error) => {
                    report.errors.push(RewardLuaDispatchError {
                        callback_id: callback.id.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };

            match callback_fn.call::<()>(context) {
                Ok(()) => match mutations.lock() {
                    Ok(mutations) => report.mutations.extend(mutations.iter().cloned()),
                    Err(error) => report.errors.push(RewardLuaDispatchError {
                        callback_id: callback.id.clone(),
                        message: error.to_string(),
                    }),
                },
                Err(error) => report.errors.push(RewardLuaDispatchError {
                    callback_id: callback.id.clone(),
                    message: error.to_string(),
                }),
            }
        }

        report
    }
}

struct RewardLuaCallback {
    #[cfg(test)]
    id: String,
    #[cfg(test)]
    key: RegistryKey,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RewardLuaDispatchReport {
    pub(super) mutations: Vec<RewardMutation>,
    pub(super) errors: Vec<RewardLuaDispatchError>,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RewardLuaDispatchError {
    pub(super) callback_id: String,
    pub(super) message: String,
}

#[cfg(test)]
pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_registry(lua, RewardLuaRegistry::new())
}

#[cfg(not(test))]
pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_registry(lua, RewardLuaRegistry::new())
}

pub(super) fn module_with_registry(lua: &Lua, registry: RewardLuaRegistry) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("on_commit", {
        let registry = registry.clone();
        lua.create_function(move |lua, callback: Function| {
            let mut callbacks = registry
                .callbacks
                .lock()
                .map_err(|_| mlua::Error::external("reward lua registry lock poisoned"))?;
            let callback_id = format!("lua:on_commit:{}", callbacks.len() + 1);
            #[cfg(test)]
            let key = lua.create_registry_value(callback.clone())?;
            callbacks.push(RewardLuaCallback {
                #[cfg(test)]
                id: callback_id.clone(),
                #[cfg(test)]
                key,
            });
            register_live_reward_callback(lua, callback_id, callback)?;
            Ok(())
        })?
    })?;
    Ok(table)
}

fn register_live_reward_callback(
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
    let live = Arc::new(Mutex::new(LiveRewardCallback {
        id: handler_id.clone(),
        lua: Lua::clone(lua),
        key: lua.create_registry_value(callback)?,
    }));

    live_bus::register_runtime_handler(handler_id, move |event| {
        let RuntimeEvent::RewardCommit(event) = event else {
            return Ok(Vec::new());
        };
        let live = live
            .lock()
            .map_err(|_| RuntimeHandlerError::new("reward lua callback lock poisoned"))?;
        live.dispatch(event)
            .map(|mutations| mutations.into_iter().map(RuntimeMutation::Reward).collect())
            .map_err(|error| RuntimeHandlerError::new(error.to_string()))
    });
    Ok(())
}

struct LiveRewardCallback {
    id: String,
    lua: Lua,
    key: RegistryKey,
}

impl LiveRewardCallback {
    fn dispatch(&self, event: &RewardCommitEvent) -> mlua::Result<Vec<RewardMutation>> {
        let callback_fn = self.lua.registry_value::<Function>(&self.key)?;
        let mutations = Arc::new(Mutex::new(Vec::new()));
        let context = reward_commit_context(&self.lua, event, Arc::clone(&mutations))?;
        callback_fn
            .call::<()>(context)
            .map_err(|error| mlua::Error::external(format!("{} failed: {error}", self.id)))?;
        let mutations = mutations
            .lock()
            .map_err(|_| mlua::Error::external("reward lua mutation lock poisoned"))?;
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

fn reward_commit_context(
    lua: &Lua,
    event: &RewardCommitEvent,
    mutations: Arc<Mutex<Vec<RewardMutation>>>,
) -> mlua::Result<Table> {
    let context = lua.create_table()?;
    context.set("rank", runtime_rank(lua, event.rank)?)?;
    context.set("rewards", runtime_rewards(lua, mutations)?)?;
    if let Some(mission_id) = event.mission_id {
        context.set("mission_id", mission_id)?;
    }
    Ok(context)
}

fn runtime_rank(lua: &Lua, rank: RankValue) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("slot", rank.slot())?;
    table.set("key", rank.key())?;
    table.set("alias", rank.debug_alias())?;
    table.set(
        "contains",
        lua.create_function(move |_lua, (_this, slots): (Table, Value)| {
            Ok(parse_rank_slots(slots)?
                .iter()
                .any(|slot| rank_matches_normalized_slot(rank, slot)))
        })?,
    )?;
    Ok(table)
}

fn runtime_rewards(lua: &Lua, mutations: Arc<Mutex<Vec<RewardMutation>>>) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("berry", runtime_berry_reward(lua, mutations)?)?;
    Ok(table)
}

fn runtime_berry_reward(
    lua: &Lua,
    mutations: Arc<Mutex<Vec<RewardMutation>>>,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(
        "multiply",
        lua.create_function(move |_, (_this, factor): (Table, Value)| {
            mutations
                .lock()
                .map_err(|_| mlua::Error::external("reward lua mutation lock poisoned"))?
                .push(RewardMutation::MultiplyBerry(parse_factor(factor)?));
            Ok(())
        })?,
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

fn parse_factor(value: Value) -> mlua::Result<f64> {
    let factor = match value {
        Value::Integer(value) => value as f64,
        Value::Number(value) => value,
        other => {
            return Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "reward multiplier".to_string(),
                message: None,
            });
        }
    };
    Ok(factor.max(1.0))
}

fn parse_rank_slots(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::Integer(slot) => Ok(vec![normalize_rank_slot(slot.to_string())]),
        Value::String(slot) => Ok(vec![normalize_rank_slot(slot.to_str()?.as_ref())]),
        Value::Table(table) => table
            .sequence_values::<Value>()
            .map(parse_rank_slot)
            .collect(),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "rank slot or rank slot list".to_string(),
            message: None,
        }),
    }
}

fn parse_rank_slot(value: mlua::Result<Value>) -> mlua::Result<String> {
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
        "5" | "s+" | "s_plus" => "s_plus".to_string(),
        slot => slot.to_string(),
    }
}
