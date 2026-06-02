use sdk_bridge::MutationEnvelope;

use crate::log;

use super::signals;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MutationApplyReport {
    pub(crate) key: String,
    pub(crate) source_mod: String,
    pub(crate) status: MutationApplyStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MutationApplyStatus {
    Applied,
    Refused { reason: String },
    Failed { reason: String },
}

pub(crate) fn apply_all(mutations: Vec<MutationEnvelope>) -> Vec<MutationApplyReport> {
    mutations.into_iter().map(apply_one).collect()
}

fn apply_one(mutation: MutationEnvelope) -> MutationApplyReport {
    let key = mutation.key.as_str().to_string();
    let source_mod = mutation.source_mod.as_str().to_string();
    let payload = mutation.payload_json;
    if !signals::has_signal_subscribers(&key) {
        return report(
            key,
            source_mod,
            MutationApplyStatus::Refused {
                reason: "no_applicator".to_string(),
            },
        );
    }
    log::write_line(format!(
        "plugin host: mutation apply requested key={key} mod={source_mod} payload_bytes={}",
        payload.len()
    ));
    let payload = serde_json::from_str::<serde_json::Value>(&payload)
        .unwrap_or_else(|_| serde_json::Value::String(payload));
    let code = signals::emit_mutation_json(
        &key,
        serde_json::json!({
            "schema": "sdk.host.mutation.v1",
            "key": key,
            "source_mod": source_mod,
            "payload": payload,
        }),
    );
    let status = if code == 0 {
        MutationApplyStatus::Applied
    } else {
        MutationApplyStatus::Failed {
            reason: format!("applicator_code_{code}"),
        }
    };
    report(key, source_mod, status)
}

fn report(key: String, source_mod: String, status: MutationApplyStatus) -> MutationApplyReport {
    MutationApplyReport {
        key,
        source_mod,
        status,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{c_char, c_void},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex, OnceLock,
        },
    };

    use sdk_bridge::{ModId, MutationEnvelope, MutationKey};

    use super::*;
    use crate::runtime::ffi::{ApiContext, CAP_SIGNALS_SUBSCRIBE};

    static ROUTED_MUTATION_BYTES: AtomicUsize = AtomicUsize::new(0);
    static ROUTED_MUTATION_PAYLOADS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

    unsafe extern "system" fn record_mutation_payload(
        _subscriber_context: *mut c_void,
        _signal_utf8: *const c_char,
        payload: *const u8,
        payload_len: usize,
    ) -> i32 {
        ROUTED_MUTATION_BYTES.store(payload_len, Ordering::Relaxed);
        if payload.is_null() && payload_len != 0 {
            return -2;
        }
        let bytes = if payload_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(payload, payload_len) }
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            return -26;
        };
        ROUTED_MUTATION_PAYLOADS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("payload store")
            .push(text.to_string());
        0
    }

    unsafe extern "system" fn fail_mutation_payload(
        _subscriber_context: *mut c_void,
        _signal_utf8: *const c_char,
        _payload: *const u8,
        _payload_len: usize,
    ) -> i32 {
        -77
    }

    #[test]
    fn mutation_registry_routes_known_signal_payloads() {
        ROUTED_MUTATION_BYTES.store(0, Ordering::Relaxed);
        ROUTED_MUTATION_PAYLOADS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("payload store")
            .clear();
        subscribe("test.runtime.mutation.apply", record_mutation_payload);

        let reports = apply_all(vec![mutation(
            "test.runtime.mutation.apply",
            serde_json::json!({ "total": 642 }).to_string(),
        )]);

        assert_eq!(reports[0].status, MutationApplyStatus::Applied);
        assert!(ROUTED_MUTATION_BYTES.load(Ordering::Relaxed) > 0);
        let payloads = ROUTED_MUTATION_PAYLOADS
            .get()
            .expect("payload store")
            .lock()
            .expect("payload store");
        let payload: serde_json::Value =
            serde_json::from_str(payloads.last().expect("mutation payload")).expect("json");
        assert_eq!(payload["schema"], "sdk.host.mutation.v1");
        assert_eq!(payload["key"], "test.runtime.mutation.apply");
        assert_eq!(payload["source_mod"], "test_mod");
        assert_eq!(payload["payload"]["total"], 642);
    }

    #[test]
    fn mutation_registry_refuses_unknown_mutation_without_crashing() {
        let reports = apply_all(vec![mutation(
            "test.runtime.mutation.unknown",
            serde_json::json!({ "total": 1 }).to_string(),
        )]);

        assert_eq!(
            reports[0].status,
            MutationApplyStatus::Refused {
                reason: "no_applicator".to_string()
            }
        );
    }

    #[test]
    fn mutation_registry_reports_applicator_failures() {
        subscribe("test.runtime.mutation.fail", fail_mutation_payload);

        let reports = apply_all(vec![mutation(
            "test.runtime.mutation.fail",
            serde_json::json!({ "total": 1 }).to_string(),
        )]);

        assert_eq!(
            reports[0].status,
            MutationApplyStatus::Failed {
                reason: "applicator_code_-77".to_string()
            }
        );
    }

    fn subscribe(signal: &str, callback: plugin_abi::HostSignalCallbackFn) {
        let context = ApiContext::new(
            "test_subscriber".to_string(),
            "mods".into(),
            [CAP_SIGNALS_SUBSCRIBE.to_string()],
            Vec::<String>::new(),
        );
        let signal = std::ffi::CString::new(signal).expect("signal");
        let code = unsafe {
            crate::runtime::signals::host_subscribe_signal(
                (&context as *const ApiContext).cast_mut().cast(),
                signal.as_ptr(),
                std::ptr::null_mut(),
                Some(callback),
            )
        };
        assert_eq!(code, 0);
    }

    fn mutation(key: &str, payload_json: String) -> MutationEnvelope {
        MutationEnvelope::new(
            MutationKey::new(key).expect("mutation key"),
            ModId::new("test_mod").expect("mod id"),
            payload_json,
        )
    }
}
