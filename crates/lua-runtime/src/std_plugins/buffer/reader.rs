use std::sync::{Arc, Mutex};

use mlua::{Lua, Table, Value};

use super::bytes::{from_lua, to_lua_table};

pub(super) fn reader(lua: &Lua, value: Value) -> mlua::Result<Table> {
    let state = Arc::new(Mutex::new(ReaderState {
        bytes: from_lua(value)?,
        position: 0,
    }));
    let table = lua.create_table()?;

    {
        let state = Arc::clone(&state);
        table.set(
            "u8",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.lock().map_err(lock_error)?;
                let bytes = state.take(1)?;
                Ok(bytes[0])
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "u16_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.lock().map_err(lock_error)?;
                let bytes = state.take(2)?;
                Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "u32_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.lock().map_err(lock_error)?;
                let bytes = state.take(4)?;
                Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "i32_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.lock().map_err(lock_error)?;
                let bytes = state.take(4)?;
                Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "bytes",
            lua.create_function(move |lua, (_self, count): (Table, usize)| {
                let mut state = state.lock().map_err(lock_error)?;
                let bytes = state.take(count)?;
                to_lua_table(lua, bytes)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "remaining",
            lua.create_function(move |_, _self: Table| {
                Ok(state.lock().map_err(lock_error)?.remaining())
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "position",
            lua.create_function(move |_, _self: Table| {
                Ok(state.lock().map_err(lock_error)?.position + 1)
            })?,
        )?;
    }
    {
        let state = Arc::clone(&state);
        table.set(
            "seek",
            lua.create_function(move |_, (_self, position): (Table, usize)| {
                if position == 0 {
                    return Err(mlua::Error::external("reader position starts at 1"));
                }
                let mut state = state.lock().map_err(lock_error)?;
                let position = position - 1;
                if position > state.bytes.len() {
                    return Err(mlua::Error::external("reader seek out of range"));
                }
                state.position = position;
                Ok(())
            })?,
        )?;
    }

    Ok(table)
}

#[derive(Debug)]
struct ReaderState {
    bytes: Vec<u8>,
    position: usize,
}

impl ReaderState {
    fn take(&mut self, count: usize) -> mlua::Result<&[u8]> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| mlua::Error::external("reader count overflow"))?;
        if end > self.bytes.len() {
            return Err(mlua::Error::external("reader out of bytes"));
        }
        let start = self.position;
        self.position = end;
        Ok(&self.bytes[start..end])
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> mlua::Error {
    mlua::Error::external("buffer reader lock poisoned")
}
