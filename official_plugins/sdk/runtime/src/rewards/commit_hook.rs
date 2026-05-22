mod format;

use std::{
    mem, panic,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::{config::RewardProbeConfig, runtime::signals};

const PLUGIN_ID: &str = "sdk_runtime";

const REWARD_COMMIT_SIGNATURE: Signature = Signature::new(
    "reward_commit_14132a670",
    &[
        0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18,
        0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xec, 0x30, 0x8b, 0xea,
    ],
    &[
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ],
);

const OVERWRITE_LEN: usize = 15;
pub(super) const REWARD_SLOT_COUNT: usize = 8;

type RewardCommitFn = extern "system" fn(*mut u64, u32, u32, i32, i32, i32) -> *mut u64;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Serialize)]
struct RewardCommitSnapshot {
    call: usize,
    reward_out: usize,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
    slots: [u64; REWARD_SLOT_COUNT],
}

pub(crate) fn install(host: OwnedHostApi, config: RewardProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "reward_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "reward_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(REWARD_COMMIT_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump(reward_commit_detour as *const () as usize)?;
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
                    "reward_probe installed site=0x{site:x} trampoline=0x{:x} max_logs={}",
                    hook.trampoline, config.max_logs
                ),
            );
        }
        Err(error) => {
            let _ = host
                .log()
                .write(PLUGIN_ID, format!("reward_probe install failed: {error}"));
        }
    }
}

extern "system" fn reward_commit_detour(
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
) -> *mut u64 {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return reward_out;
    }

    let original: RewardCommitFn = unsafe { mem::transmute(original) };
    let result = original(
        reward_out,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
    );

    let _ = panic::catch_unwind(|| {
        log_reward(
            reward_out,
            reward_param,
            mission_or_reward,
            rank_or_mode,
            bonus_a,
            bonus_b,
        );
    });

    result
}

fn log_reward(
    reward_out: *mut u64,
    reward_param: u32,
    mission_or_reward: u32,
    rank_or_mode: i32,
    bonus_a: i32,
    bonus_b: i32,
) {
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    if reward_out.is_null() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "reward_probe call={} reward_out=null param2={} param3={} param4={} param5={} param6={}",
                index + 1,
                reward_param,
                mission_or_reward,
                rank_or_mode,
                bonus_a,
                bonus_b
            ),
        );
        return;
    }

    let snapshot = format::snapshot(
        index + 1,
        reward_out,
        reward_param,
        mission_or_reward,
        rank_or_mode,
        bonus_a,
        bonus_b,
    );
    let _ = host.log().write(PLUGIN_ID, format::reward_log(&snapshot));
    signals::emit_json(host, signals::REWARD_COMMIT, &snapshot);
}
