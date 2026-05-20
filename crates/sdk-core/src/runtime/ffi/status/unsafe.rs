use std::ffi::c_void;

use plugin_abi::{Oppw4ActiveCharacter, Oppw4GameStatus};

pub(crate) unsafe extern "system" fn host_game_status(
    _host_context: *mut c_void,
    out_status: *mut Oppw4GameStatus,
) -> i32 {
    let Some(out_status) = out_status.as_mut() else {
        return -1;
    };
    super::write_game_status(out_status);
    0
}

pub(crate) unsafe extern "system" fn host_active_character(
    _host_context: *mut c_void,
    out: *mut Oppw4ActiveCharacter,
) -> i32 {
    let Some(out) = out.as_mut() else {
        return -1;
    };
    super::write_active_character(out);
    0
}

pub(crate) unsafe extern "system" fn host_debug_enabled(_host_context: *mut c_void) -> i32 {
    super::debug_enabled()
}
