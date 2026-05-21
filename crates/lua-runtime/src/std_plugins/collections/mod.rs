use mlua::Lua;

use crate::runtime::register_std_module;

mod map;
mod ring_buffer;

#[cfg(test)]
mod tests;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let collections = lua.create_table()?;
    collections.set("map", lua.create_function(map::map)?)?;
    collections.set(
        "ring_buffer",
        lua.create_function(ring_buffer::ring_buffer)?,
    )?;
    register_std_module(lua, "collections", collections)
}
