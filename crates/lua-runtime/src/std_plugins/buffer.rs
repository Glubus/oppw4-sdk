use std::{cell::RefCell, rc::Rc};

use mlua::{Lua, Table, Value};

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let buffer = lua.create_table()?;
    buffer.set("writer", lua.create_function(writer)?)?;
    buffer.set("reader", lua.create_function(reader)?)?;
    register_std_module(lua, "buffer", buffer)
}

fn writer(lua: &Lua, (): ()) -> mlua::Result<Table> {
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
                state
                    .borrow_mut()
                    .extend_from_slice(&bytes_from_lua(value)?);
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
            lua.create_function(move |lua, _self: Table| bytes_to_lua_table(lua, &state.borrow()))?,
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

fn reader(lua: &Lua, value: Value) -> mlua::Result<Table> {
    let state = Rc::new(RefCell::new(ReaderState {
        bytes: bytes_from_lua(value)?,
        position: 0,
    }));
    let table = lua.create_table()?;

    {
        let state = Rc::clone(&state);
        table.set(
            "u8",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.borrow_mut();
                let bytes = state.take(1)?;
                Ok(bytes[0])
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "u16_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.borrow_mut();
                let bytes = state.take(2)?;
                Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "u32_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.borrow_mut();
                let bytes = state.take(4)?;
                Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "i32_le",
            lua.create_function(move |_, _self: Table| {
                let mut state = state.borrow_mut();
                let bytes = state.take(4)?;
                Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "bytes",
            lua.create_function(move |lua, (_self, count): (Table, usize)| {
                let mut state = state.borrow_mut();
                let bytes = state.take(count)?;
                bytes_to_lua_table(lua, bytes)
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "remaining",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().remaining()))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "position",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().position + 1))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "seek",
            lua.create_function(move |_, (_self, position): (Table, usize)| {
                if position == 0 {
                    return Err(mlua::Error::external("reader position starts at 1"));
                }
                let mut state = state.borrow_mut();
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

fn bytes_from_lua(value: Value) -> mlua::Result<Vec<u8>> {
    match value {
        Value::String(value) => Ok(value.as_bytes().to_vec()),
        Value::Table(table) => table
            .sequence_values::<i64>()
            .map(|value| validate_u8(value?))
            .collect(),
        other => Err(mlua::Error::external(format!(
            "expected string or byte table, got {}",
            other.type_name()
        ))),
    }
}

fn bytes_to_lua_table(lua: &Lua, bytes: &[u8]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, byte) in bytes.iter().enumerate() {
        table.set(index + 1, *byte)?;
    }
    Ok(table)
}

fn validate_u8(value: i64) -> mlua::Result<u8> {
    u8::try_from(value).map_err(|_| mlua::Error::external("u8 value out of range"))
}

fn validate_u16(value: i64) -> mlua::Result<u16> {
    u16::try_from(value).map_err(|_| mlua::Error::external("u16 value out of range"))
}

fn validate_u32(value: i64) -> mlua::Result<u32> {
    u32::try_from(value).map_err(|_| mlua::Error::external("u32 value out of range"))
}

fn validate_i32(value: i64) -> mlua::Result<i32> {
    i32::try_from(value).map_err(|_| mlua::Error::external("i32 value out of range"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_lua() -> Lua {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        lua
    }

    #[test]
    fn std_buffer_is_available_through_require_and_std() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local writer = buffer.writer()
                writer:u8(1)
                return tostring(std.buffer ~= nil) .. ":" .. writer:len()
                "#,
            )
            .eval()
            .expect("std.buffer");

        assert_eq!(value, "true:1");
    }

    #[test]
    fn writer_outputs_little_endian_bytes() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local writer = buffer.writer()
                writer:u8(0x12)
                writer:u16_le(0x3456)
                writer:u32_le(0x789abcde)
                writer:i32_le(-2)
                writer:align(16, 0xff)
                local bytes = writer:to_bytes()
                return writer:len() .. ":" ..
                    bytes[1] .. ":" .. bytes[2] .. ":" .. bytes[3] .. ":" ..
                    bytes[4] .. ":" .. bytes[5] .. ":" .. bytes[6] .. ":" .. bytes[7] .. ":" ..
                    bytes[8] .. ":" .. bytes[9] .. ":" .. bytes[10] .. ":" .. bytes[16]
                "#,
            )
            .eval()
            .expect("writer");

        assert_eq!(value, "16:18:86:52:222:188:154:120:254:255:255:255");
    }

    #[test]
    fn reader_reads_little_endian_values() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local reader = buffer.reader({0x12, 0x56, 0x34, 0xde, 0xbc, 0x9a, 0x78, 0xfe, 0xff, 0xff, 0xff})
                return reader:u8() .. ":" .. reader:u16_le() .. ":" .. reader:u32_le() .. ":" ..
                    reader:i32_le() .. ":" .. reader:remaining()
                "#,
            )
            .eval()
            .expect("reader");

        assert_eq!(value, "18:13398:2023406814:-2:0");
    }

    #[test]
    fn writer_bytes_and_reader_bytes_roundtrip() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local writer = buffer.writer()
                writer:bytes({1, 2, 3})
                writer:bytes(string.char(4, 5))
                local reader = buffer.reader(writer:to_string())
                local first = reader:bytes(3)
                reader:seek(4)
                local second = reader:bytes(2)
                return first[1] .. first[2] .. first[3] .. ":" .. second[1] .. second[2] .. ":" .. reader:position()
                "#,
            )
            .eval()
            .expect("roundtrip");

        assert_eq!(value, "123:45:6");
    }

    #[test]
    fn invalid_values_are_rejected() {
        let lua = runtime_lua();

        let error = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local writer = buffer.writer()
                writer:u8(256)
                "#,
            )
            .exec()
            .expect_err("u8 should fail");

        assert!(error.to_string().contains("u8 value out of range"));
    }

    #[test]
    fn reader_rejects_out_of_bytes() {
        let lua = runtime_lua();

        let error = lua
            .load(
                r#"
                local buffer = require("std.buffer")
                local reader = buffer.reader({1})
                reader:u16_le()
                "#,
            )
            .exec()
            .expect_err("short read should fail");

        assert!(error.to_string().contains("reader out of bytes"));
    }
}
