use std::{
    panic,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

use hooks::module_base;
use plugin_sdk::OwnedHostApi;

use crate::runtime::{memory::CaveArena, probe::PLUGIN_ID};

use super::{
    labels::result_label_i32, row::write_thresholds, HOST, LOG_COUNT, LOG_ENABLED, MAX_LOGS,
    RANK_THRESHOLD_COUNT, RESULT_AGGREGATE_POST_CALL_RVA, RESULT_COUNT_CALL_RVA,
    RESULT_COUNT_POST_CALL_RVA, RESULT_SPECIAL_CAP_POST_CALL_RVA, SLOT_COUNT, SLOT_SELECTOR_OFFSET,
    THRESHOLD_ROWS,
};

const CALLSITE_COUNT_KIND: u32 = 2;
const CALLSITE_SPECIAL_CAP_KIND: u32 = 3;
const CALLSITE_AGGREGATE_KIND: u32 = 4;
const CALLSITE_COUNT_CALL_KIND: u32 = 5;
const CALLSITE_GLOBAL_COUNT_CALL_KIND: u32 = 6;
const GLOBAL_COUNT_CALL_RVA: usize = super::GLOBAL_COUNT_CALL_RVA;

static CALLSITE_COUNT_THRESHOLD_OVERRIDE: OnceLock<[u32; RANK_THRESHOLD_COUNT]> = OnceLock::new();
static CALLSITE_COUNT_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_COUNT_CALL_INSTALLED: AtomicBool = AtomicBool::new(false);
static GLOBAL_COUNT_CALL_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_SPECIAL_CAP_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_AGGREGATE_INSTALLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
enum CallsiteValue {
    Eax,
    RbxOffset(u8),
}

#[derive(Clone, Copy, Debug)]
enum ResultStateRegister {
    Rbx,
    Rdi,
}

#[derive(Clone, Copy, Debug)]
struct CallsiteProbe {
    name: &'static str,
    kind: u32,
    rva: usize,
    original: &'static [u8],
    result_state: ResultStateRegister,
    value: CallsiteValue,
    installed: &'static AtomicBool,
}

const CALLSITE_PROBES: &[CallsiteProbe] = &[
    CallsiteProbe {
        name: "result_count_post_call_14132b917",
        kind: CALLSITE_COUNT_KIND,
        rva: RESULT_COUNT_POST_CALL_RVA,
        original: &[0x89, 0x43, 0x0c, 0x89, 0x73, 0x38],
        result_state: ResultStateRegister::Rbx,
        value: CallsiteValue::RbxOffset(0x0c),
        installed: &CALLSITE_COUNT_INSTALLED,
    },
    CallsiteProbe {
        name: "result_special_cap_post_call_14132b995",
        kind: CALLSITE_SPECIAL_CAP_KIND,
        rva: RESULT_SPECIAL_CAP_POST_CALL_RVA,
        original: &[0xc7, 0x43, 0x38, 0x01, 0x00, 0x00, 0x00],
        result_state: ResultStateRegister::Rbx,
        value: CallsiteValue::RbxOffset(0x38),
        installed: &CALLSITE_SPECIAL_CAP_INSTALLED,
    },
    CallsiteProbe {
        name: "result_aggregate_post_call_14132bc75",
        kind: CALLSITE_AGGREGATE_KIND,
        rva: RESULT_AGGREGATE_POST_CALL_RVA,
        original: &[0x41, 0x89, 0x44, 0x24, 0xec],
        result_state: ResultStateRegister::Rdi,
        value: CallsiteValue::Eax,
        installed: &CALLSITE_AGGREGATE_INSTALLED,
    },
];

pub(super) fn set_count_threshold_override(thresholds: [u32; RANK_THRESHOLD_COUNT]) {
    let _ = CALLSITE_COUNT_THRESHOLD_OVERRIDE.set(thresholds);
}

pub(super) fn install_callsite_hooks(host: &OwnedHostApi) {
    let base = module_base();
    if base == 0 {
        let _ = host.log().write(
            PLUGIN_ID,
            "rank_callsite_probe install failed: module base is null",
        );
        return;
    }

    install_count_call_wrapper(
        host,
        base,
        "result_count_call_14132b912",
        RESULT_COUNT_CALL_RVA,
        [0xe8, 0x39, 0x20, 0xfb, 0xff],
        &CALLSITE_COUNT_CALL_INSTALLED,
        CALLSITE_COUNT_CALL_KIND,
    );
    install_count_call_wrapper(
        host,
        base,
        "global_count_call_14132ad34",
        GLOBAL_COUNT_CALL_RVA,
        [0xe8, 0x17, 0x2c, 0xfb, 0xff],
        &GLOBAL_COUNT_CALL_INSTALLED,
        CALLSITE_GLOBAL_COUNT_CALL_KIND,
    );
    for probe in CALLSITE_PROBES {
        install_callsite_hook(host, base, *probe);
    }
}

fn install_count_call_wrapper(
    host: &OwnedHostApi,
    base: usize,
    name: &str,
    rva: usize,
    original: [u8; 5],
    installed: &AtomicBool,
    kind: u32,
) {
    if installed.swap(true, Ordering::SeqCst) {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("rank_callsite_probe {name} already installed"),
        );
        return;
    }

    let site = base + rva;
    let mut current = [0u8; 5];
    let read = unsafe { hooks::read_memory(site, current.as_mut_ptr(), current.len()) };
    if read != 0 {
        installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!("rank_callsite_probe {name} read failed site=0x{site:x} result={read}"),
        );
        return;
    }
    if current != original {
        installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_callsite_probe {name} unexpected bytes site=0x{site:x} expected={} got={}",
                format_hex(&original),
                format_hex(&current)
            ),
        );
        return;
    }

    let result = unsafe {
        (|| -> Result<usize, String> {
            match CaveArena::new(site, 0x400) {
                Some(mut arena) => {
                    let cave = build_count_call_wrapper_cave(
                        base + 0x12dd950,
                        site + original.len(),
                        kind,
                    );
                    let cave_address = arena.alloc(&cave, 16)?;
                    let mut patch = vec![0x90; original.len()];
                    asm::write_rel32_jump(&mut patch, site, 5, cave_address)?;
                    let write = hooks::write_memory(site, patch.as_ptr(), patch.len());
                    if write == 0 {
                        Ok(cave_address)
                    } else {
                        Err(format!("write failed result={write}"))
                    }
                }
                None => Err("cave allocation failed".to_string()),
            }
        })()
    };

    match result {
        Ok(cave_address) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_callsite_probe {name} installed site=0x{site:x} cave=0x{cave_address:x}"
                ),
            );
        }
        Err(error) => {
            installed.store(false, Ordering::SeqCst);
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_callsite_probe {name} install failed site=0x{site:x}: {error}"),
            );
        }
    }
}

fn install_callsite_hook(host: &OwnedHostApi, base: usize, probe: CallsiteProbe) {
    if probe.installed.swap(true, Ordering::SeqCst) {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("rank_callsite_probe {} already installed", probe.name),
        );
        return;
    }

    let site = base + probe.rva;
    let mut current = vec![0u8; probe.original.len()];
    let read = unsafe { hooks::read_memory(site, current.as_mut_ptr(), current.len()) };
    if read != 0 {
        probe.installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_callsite_probe {} read failed site=0x{site:x} result={read}",
                probe.name
            ),
        );
        return;
    }
    if current != probe.original {
        probe.installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_callsite_probe {} unexpected bytes site=0x{site:x} expected={} got={}",
                probe.name,
                format_hex(probe.original),
                format_hex(&current)
            ),
        );
        return;
    }
    if has_rip_relative_instruction(probe.original) {
        probe.installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_callsite_probe {} refused site=0x{site:x}: original bytes contain RIP-relative instruction",
                probe.name
            ),
        );
        return;
    }

    let result = unsafe {
        (|| -> Result<usize, String> {
            match CaveArena::new(site, 0x400) {
                Some(mut arena) => {
                    let cave = build_callsite_cave(site + probe.original.len(), probe);
                    let cave_address = arena.alloc(&cave, 16)?;
                    let mut patch = vec![0x90; probe.original.len()];
                    asm::write_rel32_jump(&mut patch, site, 5, cave_address)?;
                    let write = hooks::write_memory(site, patch.as_ptr(), patch.len());
                    if write == 0 {
                        Ok(cave_address)
                    } else {
                        Err(format!("write failed result={write}"))
                    }
                }
                None => Err("cave allocation failed".to_string()),
            }
        })()
    };

    match result {
        Ok(cave_address) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_callsite_probe {} installed site=0x{site:x} cave=0x{cave_address:x}",
                    probe.name
                ),
            );
        }
        Err(error) => {
            probe.installed.store(false, Ordering::SeqCst);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_callsite_probe {} install failed site=0x{site:x}: {error}",
                    probe.name
                ),
            );
        }
    }
}

fn build_callsite_cave(return_address: usize, probe: CallsiteProbe) -> Vec<u8> {
    let mut code = Vec::new();
    code.extend_from_slice(probe.original);
    push_scratch_registers(&mut code);
    code.extend_from_slice(&[0x48, 0x83, 0xec, 0x20]);
    code.push(0xb9);
    code.extend_from_slice(&probe.kind.to_le_bytes());
    match probe.result_state {
        ResultStateRegister::Rbx => code.extend_from_slice(&[0x48, 0x8b, 0xd3]),
        ResultStateRegister::Rdi => code.extend_from_slice(&[0x48, 0x8b, 0xd7]),
    }
    match probe.value {
        CallsiteValue::Eax => code.extend_from_slice(&[0x44, 0x8b, 0xc0]),
        CallsiteValue::RbxOffset(offset) => code.extend_from_slice(&[0x44, 0x8b, 0x43, offset]),
    }
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(rank_callsite_probe_log as *const () as usize as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]);
    code.extend_from_slice(&[0x48, 0x83, 0xc4, 0x20]);
    pop_scratch_registers(&mut code);
    asm::emit_abs_jmp_r11(&mut code, return_address);
    code
}

fn build_count_call_wrapper_cave(
    helper_address: usize,
    return_address: usize,
    kind: u32,
) -> Vec<u8> {
    let mut code = Vec::new();
    push_scratch_registers(&mut code);
    code.extend_from_slice(&[0x48, 0x83, 0xec, 0x30]);
    code.extend_from_slice(&[0xf3, 0x0f, 0x11, 0x54, 0x24, 0x20]);
    code.extend_from_slice(&[0x49, 0x89, 0xca]);
    code.extend_from_slice(&[0x41, 0x89, 0xd3]);
    code.push(0xb9);
    code.extend_from_slice(&kind.to_le_bytes());
    code.extend_from_slice(&[0x4c, 0x89, 0xd2]);
    code.extend_from_slice(&[0x45, 0x8b, 0xc3]);
    code.extend_from_slice(&[0xf3, 0x0f, 0x10, 0xda]);
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(rank_count_call_probe_log as *const () as usize as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]);
    code.extend_from_slice(&[0xf3, 0x0f, 0x10, 0x54, 0x24, 0x20]);
    code.extend_from_slice(&[0x48, 0x83, 0xc4, 0x30]);
    pop_scratch_registers(&mut code);
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(helper_address as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]);
    code.extend_from_slice(&[0x0f, 0xb6, 0xc0]);
    asm::emit_abs_jmp_r11(&mut code, return_address);
    code
}

fn push_scratch_registers(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x50, 0x51, 0x52]);
    code.extend_from_slice(&[0x41, 0x50]);
    code.extend_from_slice(&[0x41, 0x51]);
    code.extend_from_slice(&[0x41, 0x52]);
    code.extend_from_slice(&[0x41, 0x53]);
    code.push(0x56);
}

fn pop_scratch_registers(code: &mut Vec<u8>) {
    code.push(0x5e);
    code.extend_from_slice(&[0x41, 0x5b]);
    code.extend_from_slice(&[0x41, 0x5a]);
    code.extend_from_slice(&[0x41, 0x59]);
    code.extend_from_slice(&[0x41, 0x58]);
    code.extend_from_slice(&[0x5a, 0x59, 0x58]);
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_rip_relative_instruction(bytes: &[u8]) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && matches!(bytes[index], 0x40..=0x4f | 0x66 | 0x67 | 0xf2 | 0xf3)
        {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        if bytes[index] == 0x0f {
            index = (index + 2).max(start + 1);
            continue;
        }

        let opcode = bytes[index];
        index += 1;
        if !opcode_uses_modrm(opcode) || index >= bytes.len() {
            continue;
        }

        let modrm = bytes[index];
        index += 1;
        if modrm & 0xc7 == 0x05 {
            return true;
        }
    }
    false
}

fn opcode_uses_modrm(opcode: u8) -> bool {
    matches!(
        opcode,
        0x00..=0x03
            | 0x08..=0x0b
            | 0x10..=0x13
            | 0x18..=0x1b
            | 0x20..=0x23
            | 0x28..=0x2b
            | 0x30..=0x33
            | 0x38..=0x3b
            | 0x62..=0x63
            | 0x69
            | 0x6b
            | 0x80..=0x8f
            | 0xc0..=0xc1
            | 0xc6..=0xc7
            | 0xd0..=0xd3
            | 0xf6..=0xf7
            | 0xfe..=0xff
    )
}

extern "system" fn rank_callsite_probe_log(kind: u32, state_ptr: usize, value: u32) {
    let _ = panic::catch_unwind(|| {
        log_rank_callsite(kind, state_ptr, value);
    });
}

extern "system" fn rank_count_call_probe_log(kind: u32, row: usize, value: u32, divisor: f32) {
    let _ = panic::catch_unwind(|| {
        log_count_callsite(kind, row, value, divisor);
    });
}

fn log_count_callsite(kind: u32, row: usize, value: u32, divisor: f32) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    let label = match kind {
        CALLSITE_COUNT_CALL_KIND => "result_count_call",
        CALLSITE_GLOBAL_COUNT_CALL_KIND => "global_count_call",
        _ => "unknown",
    };
    let selectors = [
        read_memory_u16_lossy(row + SLOT_SELECTOR_OFFSET),
        read_memory_u16_lossy(row + SLOT_SELECTOR_OFFSET + 2),
        read_memory_u16_lossy(row + SLOT_SELECTOR_OFFSET + 4),
    ];
    let count_slot = selectors
        .iter()
        .position(|selector| *selector == 1)
        .unwrap_or(usize::MAX);
    let thresholds = if count_slot < SLOT_COUNT {
        THRESHOLD_ROWS.map(|offset| read_memory_u32_lossy(row + offset + count_slot * 4))
    } else {
        [u32::MAX; RANK_THRESHOLD_COUNT]
    };
    let patched_thresholds = if count_slot < SLOT_COUNT {
        CALLSITE_COUNT_THRESHOLD_OVERRIDE
            .get()
            .copied()
            .map(|override_thresholds| {
                unsafe { write_thresholds(row, count_slot, override_thresholds) };
                override_thresholds
            })
    } else {
        None
    };
    let normalized = if divisor == 0.0 {
        u32::MAX
    } else {
        ((value as f32) / divisor) as u32
    };
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_callsite_probe kind={label} helper_row=0x{row:x} count_raw={} divisor={divisor:.3} normalized={} selectors=[{}] count_slot={} thresholds=[{}] patched_thresholds=[{}]",
            value,
            normalized,
            selectors
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(","),
            if count_slot < SLOT_COUNT {
                count_slot.to_string()
            } else {
                "none".to_string()
            },
            thresholds
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
            patched_thresholds
                .unwrap_or(thresholds)
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
}

fn log_rank_callsite(kind: u32, state_ptr: usize, value: u32) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let Some(host) = HOST.get() else {
        return;
    };
    let label = match kind {
        CALLSITE_COUNT_KIND => "result_count_post_call",
        CALLSITE_SPECIAL_CAP_KIND => "result_special_cap_post_call",
        CALLSITE_AGGREGATE_KIND => "result_aggregate_post_call",
        _ => "unknown",
    };
    if kind == CALLSITE_AGGREGATE_KIND {
        log_outer_result_callsite(host, label, state_ptr, value);
    } else {
        log_rank_block_callsite(host, label, state_ptr, value);
    }
}

fn log_rank_block_callsite(host: &OwnedHostApi, label: &str, rank_block: usize, value: u32) {
    let row_id = read_memory_u32_lossy(rank_block);
    let count_raw = read_memory_u32_lossy(rank_block + 0x04);
    let time_raw_bits = read_memory_u32_lossy(rank_block + 0x08);
    let time_raw = f32::from_bits(time_raw_bits);
    let count_rank = read_memory_u32_lossy(rank_block + 0x0c);
    let time_rank = read_memory_u32_lossy(rank_block + 0x10);
    let global_rank_copy = read_memory_u32_lossy(rank_block + 0x14);
    let rank_row_copy = read_memory_u32_lossy(rank_block + 0x18);
    let cap_flag = read_memory_u32_lossy(rank_block + 0x38);
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_callsite_probe kind={label} rank_block=0x{rank_block:x} value={}({}) row_id={} count_raw={} time_raw_bits=0x{time_raw_bits:08x} time_raw={time_raw:.3} count_rank={}({}) time_rank={}({}) global_rank_copy={}({}) rank_row_copy={} cap_flag={}",
            value,
            result_label_i32(value as i32),
            row_id,
            count_raw,
            count_rank,
            result_label_i32(count_rank as i32),
            time_rank,
            result_label_i32(time_rank as i32),
            global_rank_copy,
            result_label_i32(global_rank_copy as i32),
            rank_row_copy,
            cap_flag
        ),
    );
}

fn log_outer_result_callsite(host: &OwnedHostApi, label: &str, result_state: usize, value: u32) {
    let active_row = read_memory_u32_lossy(result_state + 0x28);
    let count_raw = read_memory_u32_lossy(result_state + 0x2c);
    let time_raw_bits = read_memory_u32_lossy(result_state + 0x30);
    let time_raw = f32::from_bits(time_raw_bits);
    let count_rank = read_memory_u32_lossy(result_state + 0x34);
    let time_rank = read_memory_u32_lossy(result_state + 0x38);
    let global_rank_copy = read_memory_u32_lossy(result_state + 0x3c);
    let rank_row_copy = read_memory_u32_lossy(result_state + 0x40);
    let aggregate_rank = read_memory_u32_lossy(result_state + 0x2a4);
    let aggregate_row = read_memory_u32_lossy(result_state + 0x330);
    let aggregate_count = read_memory_u32_lossy(result_state + 0x33c);
    let cap_flag = read_memory_u32_lossy(result_state + 0x328);
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "rank_callsite_probe kind={label} result_state=0x{result_state:x} value={}({}) active_row={} count_raw={} time_raw_bits=0x{time_raw_bits:08x} time_raw={time_raw:.3} count_rank={}({}) time_rank={}({}) global_rank_copy={}({}) rank_row_copy={} aggregate_rank={}({}) aggregate_row={} aggregate_count={} cap_flag={}",
            value,
            result_label_i32(value as i32),
            active_row,
            count_raw,
            count_rank,
            result_label_i32(count_rank as i32),
            time_rank,
            result_label_i32(time_rank as i32),
            global_rank_copy,
            result_label_i32(global_rank_copy as i32),
            rank_row_copy,
            aggregate_rank,
            result_label_i32(aggregate_rank as i32),
            aggregate_row,
            aggregate_count,
            cap_flag
        ),
    );
}

fn read_memory_u32_lossy(address: usize) -> u32 {
    let mut bytes = [0u8; 4];
    let read = unsafe { hooks::read_memory(address, bytes.as_mut_ptr(), bytes.len()) };
    if read == 0 {
        u32::from_le_bytes(bytes)
    } else {
        u32::MAX
    }
}

fn read_memory_u16_lossy(address: usize) -> u16 {
    let mut bytes = [0u8; 2];
    let read = unsafe { hooks::read_memory(address, bytes.as_mut_ptr(), bytes.len()) };
    if read == 0 {
        u16::from_le_bytes(bytes)
    } else {
        u16::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_callsite_probes_do_not_copy_rip_relative_instructions() {
        for probe in CALLSITE_PROBES {
            assert!(
                !has_rip_relative_instruction(probe.original),
                "{} contains RIP-relative bytes",
                probe.name
            );
        }
    }

    #[test]
    fn detects_removed_time_callsite_rip_relative_load() {
        let removed_time_site = [0x89, 0x43, 0x10, 0x48, 0x8b, 0x05, 0x40, 0xee, 0xb8, 0x00];

        assert!(has_rip_relative_instruction(&removed_time_site));
    }
}
