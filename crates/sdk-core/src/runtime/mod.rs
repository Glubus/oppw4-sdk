mod debug;
mod ffi;
mod linkdata;
mod loader;
mod logs;
mod lua;
mod manifest;
mod mods;
mod time;
mod win;

pub use loader::initialize;

pub fn set_debug_enabled(enabled: bool) {
    debug::set_enabled(enabled);
}
