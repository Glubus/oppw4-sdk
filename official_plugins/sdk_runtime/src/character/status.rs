use std::ffi::c_void;

use plugin_sdk::Oppw4GameStatus;

pub(crate) unsafe extern "system" fn read_game_status(
    _provider_context: *mut c_void,
    out_status: *mut Oppw4GameStatus,
) -> i32 {
    let Some(out_status) = out_status.as_mut() else {
        return -1;
    };
    *out_status = game_status_to_abi(hooks::game_status());
    0
}

fn game_status_to_abi(status: hooks::GameStatus) -> Oppw4GameStatus {
    Oppw4GameStatus {
        phase: status.phase,
        flags: status.flags,
        observed_file_opens: status.observed_file_opens,
        seconds_since_host_start: status.seconds_since_host_start,
    }
}
