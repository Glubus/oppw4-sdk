mod active_character_service;
mod debug;
mod ffi;
mod linkdata;
mod loader;
mod loader_services;
mod logs;
mod lua;
mod manifest;
mod mods;
mod time;
mod win;

pub use loader::initialize;
pub use loader_services::{
    set_active_character_reader, set_file_provider_registrar, set_game_status_reader, set_memory,
};

pub fn set_debug_enabled(enabled: bool) {
    debug::set_enabled(enabled);
}
