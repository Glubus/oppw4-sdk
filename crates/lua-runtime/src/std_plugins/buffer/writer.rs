use std::{cell::RefCell, rc::Rc};

use mlua::{Lua, Table, Value};

use super::bytes::{from_lua, to_lua_table, validate_i32, validate_u16, validate_u32, validate_u8};

pub(super) fn writer(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let state = Rc::new(RefCell::new(Vec::new()));
    let table = lua.create_table()?;

    {
        let state = Rc::clone(&state);
        table.set(
            "u8",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state.borrow_mut().push(validate_u8(value)?);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "u16_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .borrow_mut()
                    .extend_from_slice(&validate_u16(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "u32_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .borrow_mut()
                    .extend_from_slice(&validate_u32(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "i32_le",
            lua.create_function(move |_, (_self, value): (Table, i64)| {
                state
                    .borrow_mut()
                    .extend_from_slice(&validate_i32(value)?.to_le_bytes());
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "bytes",
            lua.create_function(move |_, (_self, value): (Table, Value)| {
                state.borrow_mut().extend_from_slice(&from_lua(value)?);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "align",
            lua.create_function(
                move |_, (_self, alignment, fill): (Table, usize, Option<i64>)| {
                    if alignment == 0 {
                        return Err(mlua::Error::external("alignment must be positive"));
                    }
                    let fill = validate_u8(fill.unwrap_or(0))?;
                    let mut state = state.borrow_mut();
                    let padding = (alignment - (state.len() % alignment)) % alignment;
                    state.extend(std::iter::repeat(fill).take(padding));
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "len",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().len()))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |_, _self: Table| {
                state.borrow_mut().clear();
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "to_bytes",
            lua.create_function(move |lua, _self: Table| to_lua_table(lua, &state.borrow()))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "to_string",
            lua.create_function(move |lua, _self: Table| {
                let state = state.borrow();
                lua.create_string(state.as_slice())
            })?,
        )?;
    }

    Ok(table)
}
