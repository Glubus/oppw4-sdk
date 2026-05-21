use mlua::Lua;

mod character;
mod collections;
mod files;
mod log;
mod math;
mod mod_info;
mod path;
mod time;

pub(crate) use character::authorize_extension_owner as authorize_character_extension_owner;
pub(crate) use log::collect_entries as collect_log_entries;
pub use log::LuaLogEntry;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    character::install(lua)?;
    collections::install(lua)?;
    files::install(lua)?;
    math::install(lua)?;
    mod_info::install(lua)?;
    path::install(lua)?;
    time::install(lua)?;
    log::install(lua)
}
