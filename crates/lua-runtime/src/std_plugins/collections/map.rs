use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mlua::{Lua, RegistryKey, Table, Value};

pub(super) fn map(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let state = Arc::new(Mutex::new(MapState::default()));
    let table = lua.create_table()?;

    {
        let state = Arc::clone(&state);
        table.set(
            "set",
            lua.create_function(move |lua, (_self, key, value): (Table, Value, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let registry_key = lua.create_registry_value(value)?;
                let mut state = state.lock().map_err(lock_error)?;
                if let Some(old) = state.values.insert(key.clone(), registry_key) {
                    lua.remove_registry_value(old)?;
                } else {
                    state.keys.push(key);
                }
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "get",
            lua.create_function(move |lua, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let state = state.lock().map_err(lock_error)?;
                match state.values.get(&key) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "get_or",
            lua.create_function(move |lua, (_self, key, fallback): (Table, Value, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let state = state.lock().map_err(lock_error)?;
                match state.values.get(&key) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(fallback),
                }
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "has",
            lua.create_function(move |_, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                Ok(state.lock().map_err(lock_error)?.values.contains_key(&key))
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "remove",
            lua.create_function(move |lua, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let mut state = state.lock().map_err(lock_error)?;
                state.keys.retain(|existing| existing != &key);
                match state.values.remove(&key) {
                    Some(value) => {
                        let lua_value = lua.registry_value(&value)?;
                        lua.remove_registry_value(value)?;
                        Ok(lua_value)
                    }
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |lua, _self: Table| {
                let mut state = state.lock().map_err(lock_error)?;
                for (_, value) in state.values.drain() {
                    lua.remove_registry_value(value)?;
                }
                state.keys.clear();
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "len",
            lua.create_function(move |_, _self: Table| {
                Ok(state.lock().map_err(lock_error)?.values.len())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "keys",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                let state = state.lock().map_err(lock_error)?;
                for (index, key) in state.keys.iter().enumerate() {
                    output.set(index + 1, key.to_lua_value(lua)?)?;
                }
                Ok(output)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "values",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                let state = state.lock().map_err(lock_error)?;
                for (index, key) in state.keys.iter().enumerate() {
                    if let Some(value) = state.values.get(key) {
                        output.set(index + 1, lua.registry_value::<Value>(value)?)?;
                    }
                }
                Ok(output)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "entries",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                let state = state.lock().map_err(lock_error)?;
                for (index, key) in state.keys.iter().enumerate() {
                    if let Some(value) = state.values.get(key) {
                        let entry = lua.create_table()?;
                        entry.set("key", key.to_lua_value(lua)?)?;
                        entry.set("value", lua.registry_value::<Value>(value)?)?;
                        output.set(index + 1, entry)?;
                    }
                }
                Ok(output)
            })?,
        )?;
    }

    Ok(table)
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> mlua::Error {
    mlua::Error::external("collections map lock poisoned")
}

#[derive(Debug, Default)]
struct MapState {
    values: HashMap<MapKey, RegistryKey>,
    keys: Vec<MapKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MapKey {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl MapKey {
    fn from_lua_value(value: Value) -> mlua::Result<Self> {
        match value {
            Value::String(value) => Ok(Self::String(value.to_str()?.to_string())),
            Value::Integer(value) => Ok(Self::Integer(value)),
            Value::Boolean(value) => Ok(Self::Boolean(value)),
            other => Err(mlua::Error::external(format!(
                "map key must be string, integer, or boolean, got {}",
                other.type_name()
            ))),
        }
    }

    fn to_lua_value(&self, lua: &Lua) -> mlua::Result<Value> {
        match self {
            Self::String(value) => Ok(Value::String(lua.create_string(value)?)),
            Self::Integer(value) => Ok(Value::Integer(*value)),
            Self::Boolean(value) => Ok(Value::Boolean(*value)),
        }
    }
}
