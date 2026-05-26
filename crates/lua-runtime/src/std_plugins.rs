use mlua::Lua;

mod buffer;
mod collections;
mod files;
mod log;
mod math;
mod mod_info;
mod path;
mod time;

pub(crate) use log::collect_entries as collect_log_entries;
pub use log::LuaLogEntry;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    buffer::install(lua)?;
    collections::install(lua)?;
    files::install(lua)?;
    math::install(lua)?;
    mod_info::install(lua)?;
    path::install(lua)?;
    time::install(lua)?;
    log::install(lua)
}
