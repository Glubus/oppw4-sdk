use mlua::Lua;

mod buffer;
mod character;
mod collections;
mod difficulty;
mod files;
mod json;
mod log;
mod math;
mod mission_data;
mod mod_info;
mod path;
mod ranks;
mod rewards;
mod time;

pub(crate) use character::authorize_extension_owner as authorize_character_extension_owner;
pub(crate) use log::collect_entries as collect_log_entries;
pub use log::LuaLogEntry;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    character::install(lua)?;
    buffer::install(lua)?;
    collections::install(lua)?;
    difficulty::install(lua)?;
    files::install(lua)?;
    math::install(lua)?;
    mod_info::install(lua)?;
    path::install(lua)?;
    ranks::install(lua)?;
    rewards::install(lua)?;
    time::install(lua)?;
    log::install(lua)
}
