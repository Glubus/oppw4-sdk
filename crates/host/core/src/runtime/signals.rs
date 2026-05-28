use std::{
    collections::HashMap,
    ffi::{c_char, c_void},
    sync::{Mutex, OnceLock},
};

use plugin_abi::{optional_cstr, HostSignalCallbackFn};
use sdk_bridge::{EventEnvelope, EventKey};

use crate::runtime::{
    ffi::{context_from_raw, CAP_SIGNALS_EMIT, CAP_SIGNALS_SUBSCRIBE},
    loader,
};

static SIGNALS: OnceLock<Mutex<HashMap<String, Vec<SignalSubscriber>>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct SignalSubscriber {
    context: usize,
    callback: HostSignalCallbackFn,
}

pub(crate) unsafe extern "system" fn host_subscribe_signal(
    host_context: *mut c_void,
    signal_utf8: *const c_char,
    subscriber_context: *mut c_void,
    callback: Option<HostSignalCallbackFn>,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) = context.require_capability(CAP_SIGNALS_SUBSCRIBE) {
        return code;
    }
    let Some(signal) = signal_name(signal_utf8) else {
        return -1;
    };
    let Some(callback) = callback else {
        return -2;
    };
    let Ok(mut signals) = registry().lock() else {
        return -3;
    };
    signals.entry(signal).or_default().push(SignalSubscriber {
        context: subscriber_context as usize,
        callback,
    });
    0
}

pub(crate) unsafe extern "system" fn host_emit_signal(
    host_context: *mut c_void,
    signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) = context.require_capability(CAP_SIGNALS_EMIT) {
        return code;
    }
    let Some(signal) = signal_name(signal_utf8) else {
        return -1;
    };
    if payload.is_null() && payload_len != 0 {
        return -2;
    }
    let subscribers = {
        let Ok(signals) = registry().lock() else {
            return -3;
        };
        signals.get(&signal).cloned().unwrap_or_default()
    };
    for subscriber in subscribers {
        let code = unsafe {
            (subscriber.callback)(
                subscriber.context as *mut c_void,
                signal_utf8,
                payload,
                payload_len,
            )
        };
        if code != 0 {
            return code;
        }
    }
    dispatch_runtime_event(&signal, payload, payload_len);
    0
}

fn registry() -> &'static Mutex<HashMap<String, Vec<SignalSubscriber>>> {
    SIGNALS.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn signal_name(raw: *const c_char) -> Option<String> {
    let signal = unsafe { optional_cstr(raw) }?
        .to_string_lossy()
        .trim()
        .to_string();
    (!signal.is_empty()).then_some(signal.to_ascii_lowercase())
}

fn dispatch_runtime_event(signal: &str, payload: *const u8, payload_len: usize) {
    let Ok(key) = EventKey::new(signal) else {
        return;
    };
    let payload_json = if payload_len == 0 {
        "{}".to_string()
    } else {
        let bytes = unsafe { std::slice::from_raw_parts(payload, payload_len) };
        match std::str::from_utf8(bytes) {
            Ok(value) => value.to_string(),
            Err(_) => return,
        }
    };
    let _ = loader::dispatch_event(EventEnvelope { key, payload_json });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::runtime::ffi::ApiContext;

    static SIGNAL_BYTES_SEEN: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn record_payload_len(
        _subscriber_context: *mut c_void,
        _signal_utf8: *const c_char,
        _payload: *const u8,
        payload_len: usize,
    ) -> i32 {
        SIGNAL_BYTES_SEEN.store(payload_len, Ordering::Relaxed);
        0
    }

    #[test]
    fn subscribe_signal_requires_capability() {
        let context = ApiContext::new(
            "test_plugin".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );

        let code = unsafe {
            host_subscribe_signal(
                (&context as *const ApiContext).cast_mut().cast(),
                c"test.missing_subscribe".as_ptr(),
                std::ptr::null_mut(),
                Some(record_payload_len),
            )
        };

        assert_eq!(code, -22);
    }

    #[test]
    fn emit_signal_requires_capability() {
        let context = ApiContext::new(
            "test_plugin".to_string(),
            "mods".into(),
            [CAP_SIGNALS_SUBSCRIBE.to_string()],
            Vec::<String>::new(),
        );

        let code = unsafe {
            host_emit_signal(
                (&context as *const ApiContext).cast_mut().cast(),
                c"test.missing_emit".as_ptr(),
                std::ptr::null(),
                0,
            )
        };

        assert_eq!(code, -22);
    }

    #[test]
    fn emit_signal_dispatches_to_subscribers() {
        SIGNAL_BYTES_SEEN.store(0, Ordering::Relaxed);
        let subscriber_context = ApiContext::new(
            "subscriber".to_string(),
            "mods".into(),
            [CAP_SIGNALS_SUBSCRIBE.to_string()],
            Vec::<String>::new(),
        );
        let emitter_context = ApiContext::new(
            "emitter".to_string(),
            "mods".into(),
            [CAP_SIGNALS_EMIT.to_string()],
            Vec::<String>::new(),
        );
        let signal = c"test.dispatch";
        let payload = [1_u8, 2, 3, 4];

        let subscribe_code = unsafe {
            host_subscribe_signal(
                (&subscriber_context as *const ApiContext).cast_mut().cast(),
                signal.as_ptr(),
                std::ptr::null_mut(),
                Some(record_payload_len),
            )
        };
        let emit_code = unsafe {
            host_emit_signal(
                (&emitter_context as *const ApiContext).cast_mut().cast(),
                signal.as_ptr(),
                payload.as_ptr(),
                payload.len(),
            )
        };

        assert_eq!(subscribe_code, 0);
        assert_eq!(emit_code, 0);
        assert_eq!(SIGNAL_BYTES_SEEN.load(Ordering::Relaxed), payload.len());
    }
}
