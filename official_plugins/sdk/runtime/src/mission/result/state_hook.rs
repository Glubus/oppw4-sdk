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

// Instruction boundary:
// 40 55 (2) + 41 54/55/56/57 (8) + 48 8d ac 24 20 e3 ff ff (8) = 18.
// The following mov eax,0x1de0 starts at byte 18 and must stay intact.
const OVERWRITE_LEN: usize = 18;
const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const GLOBAL_OWNER_OFFSET: usize = 0x18;
const GLOBAL_STATE_OFFSET: usize = 0x28;
const MISSION_ID_OFFSET: usize = 0x1d750;
const RESULT_MODE_OFFSET: usize = 0x1d753;
const RESULT_REWARD_MODE_OFFSET: usize = 0x1d754;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const ACTIVE_PLAYER_OFFSET: usize = 0x31;
const RANK_ROW_TABLE_OFFSET: usize = 0x1d9b0;
const RANK_ROW_STRIDE: usize = 0x50;
const PLAYER_RESULT_STRIDE: usize = 0xb90;
const PLAYER_SCORE_OFFSET: usize = 0x44c;

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

fn read_global_state() -> Result<usize, String> {
    let Some(host) = HOST.get() else {
        return Err("host unavailable".to_string());
    };
    let module_base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if module_base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_memory_usize(module_base + GLOBAL_ROOT_RVA)?;
    if root == 0 {
        return Err("global root is null".to_string());
    }
    let owner = read_memory_usize(root + GLOBAL_OWNER_OFFSET)?;
    if owner == 0 {
        return Err("global owner is null".to_string());
    }
    let state = read_memory_usize(owner + GLOBAL_STATE_OFFSET)?;
    if state == 0 {
        return Err("global state is null".to_string());
    }
    Ok(state)
}

fn read_memory_u8(address: usize) -> Result<u8, String> {
    let mut bytes = [0u8; 1];
    read_memory_bytes(address, &mut bytes)?;
    Ok(bytes[0])
}

fn read_memory_u16(address: usize) -> Result<u16, String> {
    let mut bytes = [0u8; 2];
    read_memory_bytes(address, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_memory_u32(address: usize) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    read_memory_bytes(address, &mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_memory_usize(address: usize) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    read_memory_bytes(address, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn read_memory_bytes(address: usize, out: &mut [u8]) -> Result<(), String> {
    let Some(host) = HOST.get() else {
        return Err("host unavailable".to_string());
    };
    host.memory()
        .read(address, out)
        .map_err(|error| format!("read failed address=0x{address:x}: {error}"))
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

    let source = format_rank_source().unwrap_or_else(|error| format!("rank_source={error}"));
    let _ = host.log().write(
        PLUGIN_ID,
        format!("{} {}", snapshot.format(index + 1), source),
    );
    signals::emit_json(host, signals::RESULT_STATE_SNAPSHOT, &snapshot);
}

fn format_rank_source() -> Result<String, String> {
    let global_state = read_global_state()?;
    let active_player = read_memory_u8(global_state + ACTIVE_PLAYER_OFFSET)?;
    let mission_id = read_memory_u16(global_state + MISSION_ID_OFFSET)?;
    let result_mode = read_memory_u8(global_state + RESULT_MODE_OFFSET)?;
    let reward_mode = read_memory_u8(global_state + RESULT_REWARD_MODE_OFFSET)?;
    let difficulty = read_memory_u8(global_state + DIFFICULTY_OFFSET)?;
    let rows = read_rank_rows(global_state)?;
    let player_scores = read_player_scores(global_state)?;

    Ok(format!(
        "rank_source=global=0x{global_state:x} active_player={} mission={} difficulty={} result_mode={} reward_mode={} rows={} player_scores={}",
        active_player,
        mission_id,
        difficulty,
        result_mode,
        reward_mode,
        format_rank_rows(&rows),
        format_player_scores(&player_scores),
    ))
}

fn read_rank_rows(global_state: usize) -> Result<[(u16, u16); 4], String> {
    let mut rows = [(0u16, 0u16); 4];
    for (index, row) in rows.iter_mut().enumerate() {
        let base = global_state + RANK_ROW_TABLE_OFFSET + index * RANK_ROW_STRIDE;
        *row = (read_memory_u16(base)?, read_memory_u16(base + 2)?);
    }
    Ok(rows)
}

fn read_player_scores(global_state: usize) -> Result<[[u32; 3]; 4], String> {
    let mut scores = [[0u32; 3]; 4];
    for (player, score) in scores.iter_mut().enumerate() {
        let base = global_state + player * PLAYER_RESULT_STRIDE + PLAYER_SCORE_OFFSET;
        *score = [
            read_memory_u32(base.wrapping_sub(4))?,
            read_memory_u32(base)?,
            read_memory_u32(base + 4)?,
        ];
    }
    Ok(scores)
}

fn format_rank_rows(rows: &[(u16, u16); 4]) -> String {
    rows.iter()
        .enumerate()
        .map(|(index, (rank_row, alt))| format!("p{index}:({rank_row},{alt})"))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_player_scores(scores: &[[u32; 3]; 4]) -> String {
    scores
        .iter()
        .enumerate()
        .map(|(index, values)| {
            format!(
                "p{index}:[-4:{},+0:{},+4:{}]",
                values[0], values[1], values[2]
            )
        })
        .collect::<Vec<_>>()
        .join(",")
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
