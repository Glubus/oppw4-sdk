use std::{
    ffi::{c_char, c_void},
    sync::OnceLock,
};

use plugin_sdk::OwnedHostApi;

use crate::{
    rewards::rules,
    runtime::{probe::PLUGIN_ID, signals},
};

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn install(host: OwnedHostApi) {
    let _ = HOST.set(host.clone());
    unsafe {
        subscribe(&host, signals::REWARD_STAGE_RULE);
    }
}

unsafe fn subscribe(host: &OwnedHostApi, signal: &str) {
    if let Err(error) =
        host.signals()
            .subscribe_bytes(signal, std::ptr::null_mut(), reward_command_callback)
    {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("reward_runtime subscribe failed signal={signal}: {error}"),
        );
    }
}

unsafe extern "system" fn reward_command_callback(
    _subscriber_context: *mut c_void,
    signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    let Some(host) = HOST.get() else {
        return -1;
    };
    let signal = unsafe { std::ffi::CStr::from_ptr(signal_utf8) }.to_string_lossy();
    let bytes = if payload_len == 0 {
        &[]
    } else if payload.is_null() {
        return -2;
    } else {
        unsafe { std::slice::from_raw_parts(payload, payload_len) }
    };
    match handle_command(&signal, bytes) {
        Ok(()) => 0,
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("reward_runtime command failed signal={signal}: {error}"),
            );
            -3
        }
    }
}

fn handle_command(signal: &str, payload: &[u8]) -> Result<(), String> {
    match signal {
        signals::REWARD_STAGE_RULE => {
            rules::stage(read_json(payload)?);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn read_json<'a, T>(payload: &'a [u8]) -> Result<T, String>
where
    T: serde::Deserialize<'a>,
{
    serde_json::from_slice(payload).map_err(|error| error.to_string())
}
