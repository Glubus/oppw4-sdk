use mlua::Lua;

mod character;
mod files;
mod log;
mod mod_info;

pub(crate) use character::authorize_extension_owner as authorize_character_extension_owner;
pub(crate) use log::collect_entries as collect_log_entries;
pub use log::LuaLogEntry;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    character::install(lua)?;
    files::install(lua)?;
    mod_info::install(lua)?;
    log::install(lua)
}
