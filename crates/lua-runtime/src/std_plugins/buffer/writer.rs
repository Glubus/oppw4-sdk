use std::sync::{Arc, Mutex};

use mlua::{Lua, Table, Value};

use super::bytes::{from_lua, to_lua_table, validate_i32, validate_u16, validate_u32, validate_u8};

pub(super) fn writer(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let state = Arc::new(Mutex::new(Vec::new()));
    let table = lua.create_table()?;

    {
        let state = Arc::clone(&state);
        table.set(
            "u8",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state.lock().map_err(lock_error)?.push(validate_u8(value)?);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "u16_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .lock()
                    .map_err(lock_error)?
                    .extend_from_slice(&validate_u16(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "u32_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .lock()
                    .map_err(lock_error)?
                    .extend_from_slice(&validate_u32(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "i32_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .lock()
                    .map_err(lock_error)?
                    .extend_from_slice(&validate_i32(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "bytes",
            lua.create_function(move |_, (_self, value): (Table, Value)| {
                state
                    .lock()
                    .map_err(lock_error)?
                    .extend_from_slice(&from_lua(value)?);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "align",
            lua.create_function(
                move |_, (_self, alignment, fill): (Table, usize, Option<i64>)| {
                    if alignment == 0 {
                        return Err(mlua::Error::external("alignment must be positive"));
                    }
                    let fill = validate_u8(fill.unwrap_or(0))?;
                    let mut state = state.lock().map_err(lock_error)?;
                    let padding = (alignment - (state.len() % alignment)) % alignment;
                    state.extend(std::iter::repeat(fill).take(padding));
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "len",
            lua.create_function(move |_, _self: Table| {
                Ok(state.lock().map_err(lock_error)?.len())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |_, _self: Table| {
                state.lock().map_err(lock_error)?.clear();
                Ok(())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "to_bytes",
            lua.create_function(move |lua, _self: Table| {
                let state = state.lock().map_err(lock_error)?;
                to_lua_table(lua, &state)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "to_string",
            lua.create_function(move |lua, _self: Table| {
                let state = state.lock().map_err(lock_error)?;
                lua.create_string(state.as_slice())
            })?,
        )?;
    }

    Ok(table)
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> mlua::Error {
    mlua::Error::external("buffer writer lock poisoned")
}
