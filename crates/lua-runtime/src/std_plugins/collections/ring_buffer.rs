use std::sync::{Arc, Mutex};

use mlua::{Lua, RegistryKey, Table, Value};

pub(super) fn ring_buffer(lua: &Lua, capacity: usize) -> mlua::Result<Table> {
    if capacity == 0 {
        return Err(mlua::Error::external(
            "ring_buffer capacity must be positive",
        ));
    }
    let state = Arc::new(Mutex::new(RingBufferState {
        capacity,
        values: Vec::with_capacity(capacity),
    }));
    let table = lua.create_table()?;

    {
        let state = Arc::clone(&state);
        table.set(
            "push",
            lua.create_function(move |lua, (_self, value): (Table, Value)| {
                let registry_key = lua.create_registry_value(value)?;
                let mut state = state.lock().map_err(lock_error)?;
                if state.values.len() == state.capacity {
                    let old = state.values.remove(0);
                    lua.remove_registry_value(old)?;
                }
                state.values.push(registry_key);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "last",
            lua.create_function(move |lua, _self: Table| {
                let state = state.lock().map_err(lock_error)?;
                match state.values.last() {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "get",
            lua.create_function(move |lua, (_self, index): (Table, usize)| {
                if index == 0 {
                    return Err(mlua::Error::external("ring_buffer index starts at 1"));
                }
                let state = state.lock().map_err(lock_error)?;
                match state.values.get(index - 1) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                }
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
            "capacity",
            lua.create_function(move |_, _self: Table| {
                Ok(state.lock().map_err(lock_error)?.capacity)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "is_full",
            lua.create_function(move |_, _self: Table| {
                let state = state.lock().map_err(lock_error)?;
                Ok(state.values.len() == state.capacity)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |lua, _self: Table| {
                for value in state.lock().map_err(lock_error)?.values.drain(..) {
                    lua.remove_registry_value(value)?;
                }
                Ok(())
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
                for (index, value) in state.values.iter().enumerate() {
                    output.set(index + 1, lua.registry_value::<Value>(value)?)?;
                }
                Ok(output)
            })?,
        )?;
    }

    Ok(table)
}

#[derive(Debug)]
struct RingBufferState {
    capacity: usize,
    values: Vec<RegistryKey>,
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> mlua::Error {
    mlua::Error::external("collections ring_buffer lock poisoned")
}
