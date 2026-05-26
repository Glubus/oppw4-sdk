use std::sync::{Arc, Mutex};

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::runtime::core::{
    bus::RuntimeHandlerError,
    difficulty::{
        DifficultyApplyEvent, DifficultyMutation, DifficultyValueOp as CoreDifficultyValueOp,
    },
    events::{RuntimeEvent, RuntimeMutation},
    live_bus,
};

pub(super) const MODULE_NAME: &str = "sdk.runtime.difficulty";

#[derive(Clone, Default)]
pub(super) struct DifficultyLuaRegistry {
    callbacks: Arc<Mutex<Vec<DifficultyLuaCallback>>>,
}

impl DifficultyLuaRegistry {
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
    pub(super) fn dispatch_difficulty_apply(
        &self,
        lua: &Lua,
        event: &DifficultyApplyEvent,
    ) -> DifficultyLuaDispatchReport {
        let callbacks = match self.callbacks.lock() {
            Ok(callbacks) => callbacks,
            Err(error) => {
                return DifficultyLuaDispatchReport {
                    mutations: Vec::new(),
                    errors: vec![error.to_string()],
                };
            }
        };
        let mut report = DifficultyLuaDispatchReport::default();
        for callback in callbacks.iter() {
            let callback_fn = match lua.registry_value::<Function>(&callback.key) {
                Ok(callback_fn) => callback_fn,
                Err(error) => {
                    report.errors.push(error.to_string());
                    continue;
                }
            };
            let mutations = Arc::new(Mutex::new(Vec::new()));
            let context = match difficulty_apply_context(lua, event, Arc::clone(&mutations)) {
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

struct DifficultyLuaCallback {
    #[cfg(test)]
    key: RegistryKey,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct DifficultyLuaDispatchReport {
    pub(super) mutations: Vec<DifficultyMutation>,
    pub(super) errors: Vec<String>,
}

pub(super) fn module(lua: &Lua) -> mlua::Result<Table> {
    module_with_registry(lua, DifficultyLuaRegistry::default())
}

pub(super) fn module_with_registry(
    lua: &Lua,
    registry: DifficultyLuaRegistry,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("on_apply", {
        let registry = registry.clone();
        lua.create_function(move |lua, callback: Function| {
            let mut callbacks = registry
                .callbacks
                .lock()
                .map_err(|_| mlua::Error::external("difficulty lua registry lock poisoned"))?;
            let callback_index = callbacks.len() + 1;
            #[cfg(test)]
            let key = lua.create_registry_value(callback.clone())?;
            callbacks.push(DifficultyLuaCallback {
                #[cfg(test)]
                key,
            });
            register_live_difficulty_callback(
                lua,
                format!("lua:on_apply:{callback_index}"),
                callback,
            )?;
            Ok(())
        })?
    })?;
    Ok(table)
}

fn register_live_difficulty_callback(
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
    let live = Arc::new(Mutex::new(LiveDifficultyCallback {
        id: handler_id.clone(),
        lua: Lua::clone(lua),
        key: lua.create_registry_value(callback)?,
    }));

    live_bus::register_runtime_handler(handler_id, move |event| {
        let RuntimeEvent::DifficultyApply(event) = event else {
            return Ok(Vec::new());
        };
        let live = live
            .lock()
            .map_err(|_| RuntimeHandlerError::new("difficulty lua callback lock poisoned"))?;
        live.dispatch(event)
            .map(|mutations| {
                mutations
                    .into_iter()
                    .map(RuntimeMutation::Difficulty)
                    .collect()
            })
            .map_err(|error| RuntimeHandlerError::new(error.to_string()))
    });
    Ok(())
}

struct LiveDifficultyCallback {
    id: String,
    lua: Lua,
    key: RegistryKey,
}

impl LiveDifficultyCallback {
    fn dispatch(&self, event: &DifficultyApplyEvent) -> mlua::Result<Vec<DifficultyMutation>> {
        let callback_fn = self.lua.registry_value::<Function>(&self.key)?;
        let mutations = Arc::new(Mutex::new(Vec::new()));
        let context = difficulty_apply_context(&self.lua, event, Arc::clone(&mutations))?;
        callback_fn
            .call::<()>(context)
            .map_err(|error| mlua::Error::external(format!("{} failed: {error}", self.id)))?;
        let mutations = mutations
            .lock()
            .map_err(|_| mlua::Error::external("difficulty lua mutation lock poisoned"))?;
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

fn difficulty_apply_context(
    lua: &Lua,
    event: &DifficultyApplyEvent,
    mutations: Arc<Mutex<Vec<DifficultyMutation>>>,
) -> mlua::Result<Table> {
    let context = lua.create_table()?;
    context.set("mode", event.snapshot.mode.key())?;
    context.set("difficulty", event.snapshot.difficulty.key())?;
    if let Some(mission_id) = event.snapshot.mission_id {
        context.set("mission_id", mission_id)?;
    }
    context.set("combat_pressure", runtime_combat_pressure(lua, mutations)?)?;
    Ok(context)
}

fn runtime_combat_pressure(
    lua: &Lua,
    mutations: Arc<Mutex<Vec<DifficultyMutation>>>,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(
        "multiply",
        lua.create_function(move |_, (_this, factor): (Table, Value)| {
            mutations
                .lock()
                .map_err(|_| mlua::Error::external("difficulty lua mutation lock poisoned"))?
                .push(DifficultyMutation::CombatPressure {
                    operation: CoreDifficultyValueOp::ScaleF32(parse_f32(factor)?),
                });
            Ok(())
        })?,
    )?;
    Ok(table)
}

fn parse_f32(value: Value) -> mlua::Result<f32> {
    match value {
        Value::Integer(value) => Ok(value as f32),
        Value::Number(value) => Ok(value as f32),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "number".to_string(),
            message: None,
        }),
    }
}
