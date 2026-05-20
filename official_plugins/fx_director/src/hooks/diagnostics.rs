use std::{ffi::c_void, mem, thread, time::Duration};

use crate::log;

use super::data::{read_u32, read_usize, AuraDataPtrs};

const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;

pub(super) fn start_counter_logger(data: AuraDataPtrs) {
    let _ = thread::Builder::new()
        .name("oppw4_fx_director_counters".to_string())
        .spawn(move || {
            for _ in 0..12 {
                thread::sleep(Duration::from_secs(5));
                log::write_line(format!(
                    "fx_director counters id_hits={} id_forced={} duration_hits={} duration_match={}",
                    read_u32(data.id_hits),
                    read_u32(data.id_forced_hits),
                    read_u32(data.duration_hits),
                    read_u32(data.duration_match_hits)
                ));
            }
        });
}

pub(super) fn start_observed_id_logger(data: AuraDataPtrs) {
    let _ = thread::Builder::new()
        .name("oppw4_fx_director_observed".to_string())
        .spawn(move || {
            let mut last_effect_id = u32::MAX;
            let mut last_edx = u32::MAX;
            let mut logged = 0usize;
            loop {
                thread::sleep(Duration::from_millis(250));
                if logged >= 64 {
                    log::write_line("fx_director observed original effect id log limit reached");
                    return;
                }
                let effect_id = read_u32(data.observed_effect_id);
                let edx = read_u32(data.observed_edx);
                if effect_id == 0 && edx == 0 {
                    continue;
                }
                if effect_id != last_effect_id || edx != last_edx {
                    log::write_line(format!(
                        "fx_director observed original_effect_id={} edx={}",
                        effect_id, edx
                    ));
                    logged += 1;
                    last_effect_id = effect_id;
                    last_edx = edx;
                }
            }
        });
}

pub(super) fn start_character_probe_logger(data: AuraDataPtrs) {
    let _ = thread::Builder::new()
        .name("oppw4_fx_director_character_probe".to_string())
        .spawn(move || {
            let mut last = String::new();
            let mut last_active = ActiveCharacterCandidate::default();
            loop {
                thread::sleep(Duration::from_secs(2));
                let local_player = read_usize(data.local_player);
                let local_owner = read_usize(data.local_player_fx_owner);
                let aura_param = read_usize(data.observed_aura_param);
                let aura_owner = read_usize(data.observed_aura_owner);
                let effect = read_usize(data.observed_effect);
                if local_player == 0 && aura_param == 0 {
                    continue;
                }
                let active = active_character_candidate(local_player);
                if active != last_active {
                    log::write_line(format!(
                        "fx_director active_character id={} label={} alt={} alt_label={} source={}",
                        fmt_id(active.id),
                        character_id_label(active.id),
                        fmt_id(active.alt_id),
                        character_id_label(active.alt_id),
                        fmt_ptr(active.source)
                    ));
                    last_active = active;
                }
                let snapshot = character_probe_snapshot(
                    local_player,
                    local_owner,
                    aura_param,
                    aura_owner,
                    effect,
                );
                if snapshot != last {
                    log::write_line(format!("fx_director character_probe {snapshot}"));
                    last = snapshot;
                }
            }
        });
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct ActiveCharacterCandidate {
    id: u16,
    alt_id: u16,
    source: usize,
}

fn active_character_candidate(local_player: usize) -> ActiveCharacterCandidate {
    let source = safe_read_usize(local_player + 0x118).unwrap_or(0);
    ActiveCharacterCandidate {
        id: safe_read_u16(source + 0x2).unwrap_or(u16::MAX),
        alt_id: safe_read_u16(source).unwrap_or(u16::MAX),
        source,
    }
}

fn character_probe_snapshot(
    local_player: usize,
    local_owner: usize,
    aura_param: usize,
    aura_owner: usize,
    effect: usize,
) -> String {
    let active = active_character_candidate(local_player);
    let local_118 = active.source;
    let local_460 = safe_read_usize(local_player + 0x460).unwrap_or(0);
    let owner_2c0 = safe_read_usize(local_owner + 0x2c0).unwrap_or(0);
    format!(
        "active_candidate=runtime:{}({}) alt:{}({}) source=local+0x118({}) local={} owner={} aura={} aura_owner={} effect={} lp_u32=[{}] owner_u32=[{}] lp_ptrs=[{}] owner_ptrs=[{}] nested_u32=[local+118:{} local+460:{} owner+2c0:{}] ids=[local:{} owner:{} local+118:{} local+460:{} owner+2c0:{}] aura_u32=[{}] effect_u32=[{}] aura_ptrs=[{}]",
        fmt_id(active.id),
        character_id_label(active.id),
        fmt_id(active.alt_id),
        character_id_label(active.alt_id),
        fmt_ptr(local_118),
        fmt_ptr(local_player),
        fmt_ptr(local_owner),
        fmt_ptr(aura_param),
        fmt_ptr(aura_owner),
        fmt_ptr(effect),
        probe_u32_fields(local_player, &[0x0, 0x10, 0x24, 0x28, 0x30, 0xd0, 0xd8, 0xdc, 0x118, 0x148, 0x158, 0x160, 0x280, 0x290]),
        probe_u32_fields(local_owner, &[0x0, 0x10, 0x24, 0x28, 0x30, 0x54, 0xd0, 0xd8, 0xdc, 0x118, 0x148, 0x158, 0x280, 0x290]),
        probe_ptr_fields(local_player, &[0x90, 0x118, 0x2a0, 0x2c0, 0x2d0, 0x2d8, 0x460]),
        probe_ptr_fields(local_owner, &[0x90, 0x118, 0x2a0, 0x2c0, 0x2d0, 0x2d8]),
        probe_u32_fields(local_118, &[0x0, 0x8, 0x10, 0x18, 0x20, 0x28, 0x30, 0x40, 0x48, 0x50, 0x54, 0x58, 0xb4, 0x118, 0x148, 0x280]),
        probe_u32_fields(local_460, &[0x0, 0x10, 0x24, 0x28, 0x30, 0x54, 0xd0, 0xd8, 0xdc, 0x118, 0x148, 0x158, 0x280, 0x290]),
        probe_u32_fields(owner_2c0, &[0x0, 0x8, 0x10, 0x18, 0x20, 0x28, 0x30, 0x40, 0x48, 0x50, 0x54, 0x58, 0xb4, 0x118, 0x148, 0x280]),
        scan_known_ids(local_player, 0x320),
        scan_known_ids(local_owner, 0x320),
        scan_known_ids(local_118, 0x320),
        scan_known_ids(local_460, 0x320),
        scan_known_ids(owner_2c0, 0x320),
        probe_u32_fields(aura_param, &[0x0, 0x10, 0x24, 0x28, 0x30, 0xd0, 0xd8, 0xdc, 0x280, 0x290]),
        probe_u32_fields(effect, &[0x0, 0x10, 0x30, 0x54, 0x58, 0x5c, 0x60]),
        probe_ptr_fields(aura_param, &[0x28, 0x38, 0x410, 0x458, 0x2d0, 0x2d8])
    )
}

fn fmt_ptr(value: usize) -> String {
    if value == 0 {
        "0".to_string()
    } else {
        format!("0x{value:x}")
    }
}

fn probe_u32_fields(base: usize, offsets: &[usize]) -> String {
    if base == 0 {
        return "none".to_string();
    }
    offsets
        .iter()
        .map(|offset| {
            let value = safe_read_u32(base + offset)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unreadable".to_string());
            format!("+0x{offset:x}:{value}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn probe_ptr_fields(base: usize, offsets: &[usize]) -> String {
    if base == 0 {
        return "none".to_string();
    }
    offsets
        .iter()
        .map(|offset| {
            let value = safe_read_usize(base + offset)
                .map(fmt_ptr)
                .unwrap_or_else(|| "unreadable".to_string());
            format!("+0x{offset:x}:{value}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn scan_known_ids(base: usize, len: usize) -> String {
    if base == 0 {
        return "none".to_string();
    }
    let mut hits = Vec::new();
    for offset in (0..len).step_by(2) {
        let Some(id) = safe_read_u16(base + offset) else {
            continue;
        };
        if id == 0 || id == u16::MAX {
            continue;
        }
        let label = character_id_label(id);
        if label == "unknown" {
            continue;
        }
        hits.push(format!("+0x{offset:x}:{id}({label})"));
        if hits.len() >= 16 {
            break;
        }
    }
    if hits.is_empty() {
        "none".to_string()
    } else {
        hits.join(",")
    }
}

fn fmt_id(id: u16) -> String {
    if id == u16::MAX {
        "none".to_string()
    } else {
        id.to_string()
    }
}

fn character_id_label(id: u16) -> String {
    if id == u16::MAX {
        return "none".to_string();
    }
    for character in struct_api::all() {
        if character.runtime_id == Some(id) {
            return format!("{}:runtime_id", character.canonical);
        }
        if character.boss_runtime_id == Some(id) {
            return format!("{}:boss_runtime_id", character.canonical);
        }
        if character.playable_id == Some(id) {
            return format!("{}:playable_id", character.canonical);
        }
        if character.model_id == Some(id) {
            return format!("{}:model_id", character.canonical);
        }
    }
    "unknown".to_string()
}

fn safe_read_u16(address: usize) -> Option<u16> {
    if !is_readable_range(address, mem::size_of::<u16>()) {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(address as *const u16) })
}

fn safe_read_u32(address: usize) -> Option<u32> {
    if !is_readable_range(address, mem::size_of::<u32>()) {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(address as *const u32) })
}

fn safe_read_usize(address: usize) -> Option<usize> {
    if !is_readable_range(address, mem::size_of::<usize>()) {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(address as *const usize) })
}

fn is_readable_range(address: usize, len: usize) -> bool {
    if address == 0 || len == 0 {
        return false;
    }
    let Some(end) = address.checked_add(len.saturating_sub(1)) else {
        return false;
    };
    let mut cursor = address;
    while cursor <= end {
        let Some(region) = (unsafe { query_memory(cursor) }) else {
            return false;
        };
        if region.state != MEM_COMMIT
            || region.protect & (PAGE_NOACCESS | PAGE_GUARD) != 0
            || region.region_size == 0
        {
            return false;
        }
        let region_end = region.base_address.saturating_add(region.region_size);
        if region_end == 0 || region_end <= cursor {
            return false;
        }
        if region_end > end {
            return true;
        }
        cursor = region_end;
    }
    true
}

unsafe fn query_memory(address: usize) -> Option<MemoryBasicInformation> {
    let mut info = MemoryBasicInformation::default();
    let written = VirtualQuery(
        address as *const c_void,
        (&mut info as *mut MemoryBasicInformation).cast(),
        mem::size_of::<MemoryBasicInformation>(),
    );
    (written != 0).then_some(info)
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryBasicInformation {
    base_address: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition_id: u16,
    _alignment: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    type_: u32,
}

extern "system" {
    fn VirtualQuery(address: *const c_void, buffer: *mut c_void, length: usize) -> usize;
}
