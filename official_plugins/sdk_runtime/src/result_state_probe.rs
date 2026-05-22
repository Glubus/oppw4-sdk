use std::{
    fmt::Write,
    mem, panic, slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::config::ResultStateProbeConfig;

const PLUGIN_ID: &str = "sdk_runtime";

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
const U32_BLOCK_WORDS: usize = 14;
const CREW_POINT_BLOCK_OFFSET: usize = 0x498;
const CREW_POINT_BLOCK_WORDS: usize = 14;
const MAX_CREW_UNLOCKS: usize = 32;
const CREW_UNLOCK_WORDS: usize = 2;
const EVENT_WORDS: usize = 5;
const MAX_EVENT_BYTES: usize = 0x4000;

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
        Ok((site, hook)) => {
            TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let _ = HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "result_state_probe installed site=0x{site:x} trampoline=0x{:x} max_logs={} max_events={}",
                    hook.trampoline,
                    config.max_logs,
                    config.max_events,
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("result_state_probe install failed: {error}"),
            );
        }
    }
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
    let Some(snapshot) = (unsafe { ResultStateSnapshot::read(result_state) }) else {
        return;
    };
    if !snapshot.should_log() {
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
}

#[derive(Debug)]
struct ResultStateSnapshot {
    address: usize,
    state: u32,
    unlock_flags: u32,
    mission_id: u32,
    difficulty_or_rank: [u32; 5],
    result_copy: [u32; U32_BLOCK_WORDS],
    crew_points: [u32; CREW_POINT_BLOCK_WORDS],
    source_rewards: [u32; U32_BLOCK_WORDS],
    soul_counter: u32,
    character_rewards: [u32; 5],
    character_totals: [u32; 5],
    crew_unlock_count: u32,
    crew_unlocks: Vec<[u32; CREW_UNLOCK_WORDS]>,
    event_count: usize,
    events: Vec<[u32; EVENT_WORDS]>,
    hash: u64,
}

impl ResultStateSnapshot {
    unsafe fn read(result_state: *const u8) -> Option<Self> {
        if result_state.is_null() {
            return None;
        }

        let result_copy = read_u32_block::<U32_BLOCK_WORDS>(result_state, 0x2ac);
        let crew_points =
            read_u32_block::<CREW_POINT_BLOCK_WORDS>(result_state, CREW_POINT_BLOCK_OFFSET);
        let source_rewards = read_u32_block::<U32_BLOCK_WORDS>(result_state, 0x7a0);
        let crew_unlock_count = read_u32(result_state, 0x920);
        let crew_unlocks = read_crew_unlocks(
            result_state,
            crew_unlock_count.min(MAX_CREW_UNLOCKS as u32) as usize,
        );
        let events = read_events(result_state, MAX_EVENTS.load(Ordering::Relaxed));
        let event_count = events.len();
        let character_rewards = read_u32_block::<5>(result_state, 0x2ec);
        let character_totals = read_u32_block::<5>(result_state, 0x300);

        let mut snapshot = Self {
            address: result_state as usize,
            state: read_u32(result_state, 0x10),
            unlock_flags: read_u32(result_state, 0x24),
            mission_id: read_u32(result_state, 0x40),
            difficulty_or_rank: read_u32_block::<5>(result_state, 0x28),
            result_copy,
            crew_points,
            source_rewards,
            soul_counter: read_u32(result_state, 0x2e8),
            character_rewards,
            character_totals,
            crew_unlock_count,
            crew_unlocks,
            event_count,
            events,
            hash: 0,
        };
        snapshot.hash = snapshot.calculate_hash();
        Some(snapshot)
    }

    fn should_log(&self) -> bool {
        let mut last_hash = LAST_HASH.lock().expect("result_state_probe hash mutex");
        if *last_hash == self.hash {
            return false;
        }
        *last_hash = self.hash;
        true
    }

    fn calculate_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        hash_u32(&mut hash, self.state);
        hash_u32(&mut hash, self.unlock_flags);
        hash_u32(&mut hash, self.mission_id);
        for value in self
            .difficulty_or_rank
            .iter()
            .chain(self.result_copy.iter())
            .chain(self.crew_points.iter())
            .chain(self.source_rewards.iter())
            .chain(self.character_rewards.iter())
            .chain(self.character_totals.iter())
        {
            hash_u32(&mut hash, *value);
        }
        hash_u32(&mut hash, self.soul_counter);
        hash_u32(&mut hash, self.crew_unlock_count);
        for pair in &self.crew_unlocks {
            hash_u32(&mut hash, pair[0]);
            hash_u32(&mut hash, pair[1]);
        }
        for event in &self.events {
            for value in event {
                hash_u32(&mut hash, *value);
            }
        }
        hash
    }

    fn format(&self, call: usize) -> String {
        format!(
            "result_state_probe call={call} ptr=0x{:x} state={} mission={} rank_fields=[{}] flags=0x{:x} result_copy=[{}] crew_points=[{}] crew_points_named={} source_rewards=[{}] soul_counter={} character_rewards=[{}] character_totals=[{}] crew_unlock_count={} crew_unlocks={} event_count={} events={}",
            self.address,
            self.state,
            self.mission_id,
            format_array(&self.difficulty_or_rank),
            self.unlock_flags,
            format_array(&self.result_copy),
            format_array(&self.crew_points),
            format_offset_block(&self.crew_points, CREW_POINT_BLOCK_OFFSET),
            format_array(&self.source_rewards),
            self.soul_counter,
            format_array(&self.character_rewards),
            format_array(&self.character_totals),
            self.crew_unlock_count,
            format_pairs(&self.crew_unlocks),
            self.event_count,
            format_events(&self.events),
        )
    }
}

unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
    (base.add(offset) as *const u32).read_unaligned()
}

unsafe fn read_u64(base: *const u8, offset: usize) -> u64 {
    (base.add(offset) as *const u64).read_unaligned()
}

unsafe fn read_u32_block<const N: usize>(base: *const u8, offset: usize) -> [u32; N] {
    let mut values = [0u32; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(base, offset + index * 4);
    }
    values
}

unsafe fn read_crew_unlocks(base: *const u8, count: usize) -> Vec<[u32; CREW_UNLOCK_WORDS]> {
    let mut unlocks = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 0x820 + index * 8;
        unlocks.push([read_u32(base, offset), read_u32(base, offset + 4)]);
    }
    unlocks
}

unsafe fn read_events(base: *const u8, limit: usize) -> Vec<[u32; EVENT_WORDS]> {
    let begin = read_u64(base, 0xb10) as usize;
    let end = read_u64(base, 0xb18) as usize;
    if begin == 0 || end < begin {
        return Vec::new();
    }

    let byte_len = end - begin;
    if byte_len > MAX_EVENT_BYTES {
        return Vec::new();
    }

    let entry_count = (byte_len / (EVENT_WORDS * 4)).min(limit);
    let words = slice::from_raw_parts(begin as *const u32, entry_count * EVENT_WORDS);
    words
        .chunks_exact(EVENT_WORDS)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]])
        .collect()
}

fn format_array(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_offset_block(values: &[u32], start_offset: usize) -> String {
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "+0x{:x}:{}", start_offset + index * 4, value);
    }
    text
}

fn format_pairs(values: &[[u32; CREW_UNLOCK_WORDS]]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    let mut text = String::new();
    for (index, pair) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "#{}:[{},{}]", index, pair[0], pair[1]);
    }
    text
}

fn format_events(values: &[[u32; EVENT_WORDS]]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    let mut text = String::new();
    for (index, event) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(
            text,
            "#{}:[{},{},{},{},{}]",
            index, event[0], event[1], event[2], event[3], event[4]
        );
    }
    text
}

fn hash_u32(hash: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_empty_collections() {
        assert_eq!(format_pairs(&[]), "none");
        assert_eq!(format_events(&[]), "none");
    }

    #[test]
    fn formats_pairs_and_events() {
        assert_eq!(format_pairs(&[[1, 2], [3, 4]]), "#0:[1,2],#1:[3,4]");
        assert_eq!(format_events(&[[1, 2, 3, 4, 5]]), "#0:[1,2,3,4,5]");
    }

    #[test]
    fn formats_offset_blocks() {
        assert_eq!(
            format_offset_block(&[12, 34, 56], 0x498),
            "+0x498:12,+0x49c:34,+0x4a0:56"
        );
    }
}
