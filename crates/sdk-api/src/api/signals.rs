use std::ffi::c_void;

use plugin_abi::{HostSignalCallbackFn, Oppw4PluginApi};

use crate::{error::PluginError, helpers::cstring_lossy, PluginResult};

#[derive(Clone, Copy)]
pub struct SignalService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> SignalService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    /// # Safety
    ///
    /// `subscriber_context` and `callback` must remain valid while the plugin
    /// is loaded. The callback must not retain the signal or payload pointers
    /// after returning.
    pub unsafe fn subscribe_bytes(
        self,
        signal: &str,
        subscriber_context: *mut c_void,
        callback: HostSignalCallbackFn,
    ) -> PluginResult<()> {
        let subscribe = self
            .abi
            .subscribe_signal
            .ok_or(PluginError::MissingHostFunction("subscribe_signal"))?;
        let signal = cstring_lossy(signal);
        let code = super::r#unsafe::subscribe_signal(
            self.abi.host_context,
            subscribe,
            signal.as_c_str(),
            subscriber_context,
            callback,
        );
        host_code_result("subscribe_signal", code)
    }

    pub fn emit_bytes(self, signal: &str, payload: &[u8]) -> PluginResult<()> {
        let emit = self
            .abi
            .emit_signal
            .ok_or(PluginError::MissingHostFunction("emit_signal"))?;
        let signal = cstring_lossy(signal);
        let code =
            super::r#unsafe::emit_signal(self.abi.host_context, emit, signal.as_c_str(), payload);
        host_code_result("emit_signal", code)
    }
}

fn host_code_result(operation: &'static str, code: i32) -> PluginResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(PluginError::HostCallFailed { operation, code })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{c_char, c_void},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use plugin_abi::null_api;

    use super::*;

    static LAST_PAYLOAD_LEN: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn subscribe_signal(
        _host_context: *mut c_void,
        _signal_utf8: *const c_char,
        _subscriber_context: *mut c_void,
        callback: Option<HostSignalCallbackFn>,
    ) -> i32 {
        if callback.is_some() {
            0
        } else {
            -1
        }
    }

    unsafe extern "system" fn emit_signal(
        _host_context: *mut c_void,
        _signal_utf8: *const c_char,
        _payload: *const u8,
        payload_len: usize,
    ) -> i32 {
        LAST_PAYLOAD_LEN.store(payload_len, Ordering::Relaxed);
        0
    }

    unsafe extern "system" fn signal_callback(
        _subscriber_context: *mut c_void,
        _signal_utf8: *const c_char,
        _payload: *const u8,
        _payload_len: usize,
    ) -> i32 {
        0
    }

    #[test]
    fn signal_service_subscribes() {
        let mut api = null_api();
        api.subscribe_signal = Some(subscribe_signal);

        let result = unsafe {
            SignalService::new(&api).subscribe_bytes(
                "runtime.loaded",
                std::ptr::null_mut(),
                signal_callback,
            )
        };

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn signal_service_emits_bytes() {
        LAST_PAYLOAD_LEN.store(0, Ordering::Relaxed);
        let mut api = null_api();
        api.emit_signal = Some(emit_signal);

        let result = SignalService::new(&api).emit_bytes("runtime.loaded", &[1, 2, 3]);

        assert_eq!(result, Ok(()));
        assert_eq!(LAST_PAYLOAD_LEN.load(Ordering::Relaxed), 3);
    }
}
