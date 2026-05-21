use mlua::Lua;

use crate::runtime::register_std_module;

mod bytes;
mod reader;
mod writer;

#[cfg(test)]
mod tests;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let buffer = lua.create_table()?;
    buffer.set("writer", lua.create_function(writer::writer)?)?;
    buffer.set("reader", lua.create_function(reader::reader)?)?;
    register_std_module(lua, "buffer", buffer)
}
