use mlua::Lua;

mod buffer;
mod collections;
mod files;
mod log;
mod math;
mod mod_info;
mod path;
mod time;

pub use log::LuaLogEntry;
pub(crate) use log::{clear_entries as clear_log_entries, collect_entries as collect_log_entries};

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
