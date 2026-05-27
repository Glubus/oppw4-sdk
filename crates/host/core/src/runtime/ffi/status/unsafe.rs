use std::ffi::c_void;

use plugin_abi::{Oppw4ActiveCharacter, Oppw4GameStatus};

pub(crate) unsafe extern "system" fn host_register_game_status_provider(
    _host_context: *mut c_void,
    provider_context: *mut c_void,
    callback: Option<plugin_abi::HostGameStatusFn>,
) -> i32 {
    super::register_game_status_provider(provider_context, callback)
}

pub(crate) unsafe extern "system" fn host_register_active_character_provider(
    _host_context: *mut c_void,
    provider_context: *mut c_void,
    callback: Option<plugin_abi::HostActiveCharacterFn>,
) -> i32 {
    super::register_active_character_provider(provider_context, callback)
}

pub(crate) unsafe extern "system" fn host_game_status(
    _host_context: *mut c_void,
    out_status: *mut Oppw4GameStatus,
) -> i32 {
    let Some(out_status) = out_status.as_mut() else {
        return -1;
    };
    super::read_game_status(out_status)
}

pub(crate) unsafe extern "system" fn host_active_character(
    _host_context: *mut c_void,
    out: *mut Oppw4ActiveCharacter,
) -> i32 {
    let Some(out) = out.as_mut() else {
        return -1;
    };
    super::read_active_character(out)
}

pub(crate) unsafe extern "system" fn host_debug_enabled(_host_context: *mut c_void) -> i32 {
    super::debug_enabled()
}
