use std::{
    ffi::{CStr, CString},
    path::PathBuf,
    sync::OnceLock,
};

use plugin_abi::{Oppw4LoaderLogFn, Oppw4LoaderSdkInit, OPPW4_LOADER_SDK_ABI_VERSION};

static LOADER_LOG: OnceLock<LoaderLog> = OnceLock::new();

#[derive(Clone, Copy)]
struct LoaderLog {
    context: usize,
    callback: Oppw4LoaderLogFn,
}

#[no_mangle]
pub unsafe extern "system" fn oppw4_sdk_core_initialize(init: *const Oppw4LoaderSdkInit) -> i32 {
    let Some(init) = init.as_ref() else {
        return -1;
    };
    if init.version != OPPW4_LOADER_SDK_ABI_VERSION {
        return -2;
    }

    if let Some(callback) = init.log {
        let _ = LOADER_LOG.set(LoaderLog {
            context: init.host_context as usize,
            callback,
        });
        plugin_host::set_logger(forward_log_to_loader);
    }
    plugin_host::set_debug_enabled(init.debug_enabled != 0);
    if let Some(register_file_provider) = init.register_file_provider {
        plugin_host::set_file_provider_registrar(init.host_context, register_file_provider);
    }
    if let Some(game_status) = init.game_status {
        plugin_host::set_game_status_reader(init.host_context, game_status);
    }
    if let Some(active_character) = init.active_character {
        plugin_host::set_active_character_reader(init.host_context, active_character);
    }

    let Some(game_root) = path_from_cstr(init.game_root_utf8) else {
        return -3;
    };
    let Some(plugin_root) = path_from_cstr(init.plugin_root_utf8) else {
        return -4;
    };
    let session_stamp = optional_string_from_cstr(init.session_stamp_utf8);

    plugin_host::initialize(&game_root, &plugin_root, session_stamp);
    0
}

fn path_from_cstr(raw: *const std::ffi::c_char) -> Option<PathBuf> {
    string_from_cstr(raw).map(PathBuf::from)
}

fn optional_string_from_cstr(raw: *const std::ffi::c_char) -> Option<String> {
    if raw.is_null() {
        None
    } else {
        string_from_cstr(raw)
    }
}

fn string_from_cstr(raw: *const std::ffi::c_char) -> Option<String> {
    let value = unsafe { CStr::from_ptr(raw) }
        .to_str()
        .ok()?
        .trim()
        .to_string();
    (!value.is_empty()).then_some(value)
}

fn forward_log_to_loader(message: String) {
    let Some(log) = LOADER_LOG.get() else {
        return;
    };
    let Ok(message) = CString::new(message) else {
        return;
    };
    unsafe {
        (log.callback)(log.context as *mut _, message.as_ptr());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn empty_optional_session_stamp_is_none() {
        let value = CString::new("").expect("empty");

        assert_eq!(optional_string_from_cstr(value.as_ptr()), None);
    }

    #[test]
    fn path_from_cstr_rejects_invalid_utf8() {
        let value = [0xff, 0x00];

        assert_eq!(path_from_cstr(value.as_ptr().cast()), None);
    }
}
