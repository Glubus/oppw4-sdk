mod format;
mod hash;
mod reader;
mod snapshot;

use std::{
    mem, panic,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::{
    config::ResultStateProbeConfig,
    runtime::{probe::PLUGIN_ID, signals},
};

const RESULT_STATE_SIGNATURE: Signature = Signature::new(
    "result_state_14132b570",
    &[
        0x40, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x20,
        0xe3, 0xff, 0xff, 0xb8, 0xe0, 0x1d, 0x00, 0x00,
    ],
    &[
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ],
);

const OVERWRITE_LEN: usize = 18;

type ResultStateFn = extern "system" fn(*mut u8);

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static MAX_EVENTS: AtomicUsize = AtomicUsize::new(0);
static LAST_HASH: Mutex<u64> = Mutex::new(0);

pub(crate) fn install(host: OwnedHostApi, config: ResultStateProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "result_state_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
    MAX_EVENTS.store(config.max_events, Ordering::Relaxed);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "result_state_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(RESULT_STATE_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump(result_state_detour as *const () as usize)?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => finish_install(host, site, hook, config),
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("result_state_probe install failed: {error}"),
            );
        }
    }
}

fn finish_install(
    host: OwnedHostApi,
    site: usize,
    hook: InlineHook,
    config: ResultStateProbeConfig,
) {
    TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
    let _ = HOOK.set(hook);
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "result_state_probe installed site=0x{site:x} trampoline=0x{:x} max_logs={} max_events={}",
            TRAMPOLINE.load(Ordering::SeqCst),
            config.max_logs,
            config.max_events,
        ),
    );
}

extern "system" fn result_state_detour(result_state: *mut u8) {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return;
    }

    let original: ResultStateFn = unsafe { mem::transmute(original) };
    original(result_state);

    let _ = panic::catch_unwind(|| {
        log_result_state(result_state);
    });
}

fn log_result_state(result_state: *mut u8) {
    let Some(snapshot) =
        (unsafe { snapshot::ResultStateSnapshot::read(result_state, max_events()) })
    else {
        return;
    };
    if !should_log(snapshot.hash()) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let _ = host.log().write(PLUGIN_ID, snapshot.format(index + 1));
    signals::emit_json(host, signals::RESULT_STATE_SNAPSHOT, &snapshot);
}

fn should_log(hash: u64) -> bool {
    let mut last_hash = LAST_HASH.lock().expect("result_state_probe hash mutex");
    if *last_hash == hash {
        return false;
    }
    *last_hash = hash;
    true
}

fn max_events() -> usize {
    MAX_EVENTS.load(Ordering::Relaxed)
}
