mod config;
mod debug;
mod ffi;
mod linkdata;
mod loader;
mod loader_services;
mod logs;
mod lua;
mod manifest;
mod mods;
mod registry;
mod rdb;
mod signals;
mod time;
mod win;

pub use loader::initialize;
pub use loader_services::{set_file_provider_registrar, set_memory};

pub fn set_debug_enabled(enabled: bool) {
    debug::set_enabled(enabled);
}
