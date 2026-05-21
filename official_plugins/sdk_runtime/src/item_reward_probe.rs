use std::{
    fmt::Write,
    mem, panic, slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::config::ItemRewardProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

const ITEM_REWARD_SIGNATURE: Signature = Signature::new(
    "item_reward_14132d280",
    &[
        0x40, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8d, 0xac, 0x24, 0x80,
        0xed, 0xff, 0xff, 0xb8, 0x80, 0x13, 0x00, 0x00,
    ],
    &[
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ],
);

const OVERWRITE_LEN: usize = 18;
const TRIPLET_WORDS: usize = 3;
const MAX_GAME_ENTRIES: usize = 40;

type ItemRewardFn = extern "system" fn(*mut i32, u64, *const i32) -> u32;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static MAX_ENTRIES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn install(host: OwnedHostApi, config: ItemRewardProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "item_reward_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
    MAX_ENTRIES.store(config.max_entries.min(MAX_GAME_ENTRIES), Ordering::Relaxed);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "item_reward_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(ITEM_REWARD_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump(item_reward_detour as usize)?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let _ = HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "item_reward_probe installed site=0x{site:x} trampoline=0x{:x} max_logs={} max_entries={}",
                    hook.trampoline,
                    config.max_logs,
                    config.max_entries.min(MAX_GAME_ENTRIES)
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("item_reward_probe install failed: {error}"),
            );
        }
    }
}

extern "system" fn item_reward_detour(
    out: *mut i32,
    reward_context: u64,
    previous: *const i32,
) -> u32 {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return 0;
    }

    let original: ItemRewardFn = unsafe { mem::transmute(original) };
    let result = original(out, reward_context, previous);

    let _ = panic::catch_unwind(|| {
        log_items(out, reward_context, previous, result);
    });

    result
}

fn log_items(out: *mut i32, reward_context: u64, previous: *const i32, result: u32) {
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    if out.is_null() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "item_reward_probe call={} out=null context={} previous=0x{:x} result={}",
                index + 1,
                reward_context,
                previous as usize,
                result
            ),
        );
        return;
    }

    let max_entries = MAX_ENTRIES
        .load(Ordering::Relaxed)
        .clamp(1, MAX_GAME_ENTRIES);
    let words =
        unsafe { slice::from_raw_parts(out.cast_const(), MAX_GAME_ENTRIES * TRIPLET_WORDS) };
    let entries = format_entries(words, max_entries);
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "item_reward_probe call={} out=0x{:x} context={} previous=0x{:x} result={} entries={entries}",
            index + 1,
            out as usize,
            reward_context,
            previous as usize,
            result,
        ),
    );
}

fn format_entries(words: &[i32], max_entries: usize) -> String {
    let mut text = String::new();
    let mut written = 0usize;
    for (index, entry) in words.chunks_exact(TRIPLET_WORDS).enumerate() {
        let amount = entry[0];
        if amount == 0 {
            continue;
        }
        if written > 0 {
            text.push(',');
        }
        let item_id = entry[1];
        let is_new = entry[2];
        let _ = write!(text, "#{index}:amount={amount}:item={item_id}:new={is_new}");
        written += 1;
        if written >= max_entries {
            break;
        }
    }
    if text.is_empty() {
        "none".to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_non_zero_triplets() {
        let words = [0, 0, 0, 5, 73, 1, 7, 31, 0];

        assert_eq!(
            format_entries(&words, 40),
            "#1:amount=5:item=73:new=1,#2:amount=7:item=31:new=0"
        );
    }
}
