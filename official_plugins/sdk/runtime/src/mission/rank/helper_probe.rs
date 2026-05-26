use std::{
    mem, panic,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{module_base, HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::{
    config::{RankHelperProbeConfig, RankRuntimeConfig},
    runtime::memory::CaveArena,
    runtime::{probe::PLUGIN_ID, signals},
};

const TIME_RANK_SIGNATURE: Signature = Signature::new(
    "rank_time_helper_1412dd9e0",
    &[
        0x4c, 0x8d, 0x41, 0x64, 0x0f, 0x28, 0xd1, 0x48, 0x8d, 0x51, 0x6a, 0x49, 0x8b, 0xc0, 0x4c,
        0x3b, 0xc2, 0x74, 0x0f, 0x66, 0x83, 0x38, 0x00, 0x74, 0x09, 0x48, 0x83, 0xc0, 0x02, 0x48,
        0x3b, 0xc2,
    ],
    &[1; 32],
);

const COUNT_RANK_SIGNATURE: Signature = Signature::new(
    "rank_count_helper_1412dd950",
    &[
        0x4c, 0x8d, 0x49, 0x64, 0x48, 0x8d, 0x41, 0x6a, 0x4d, 0x8b, 0xc1, 0x4c, 0x3b, 0xc8, 0x74,
        0x10, 0x66, 0x41, 0x83, 0x38, 0x01, 0x74, 0x09, 0x49, 0x83, 0xc0, 0x02, 0x4c, 0x3b, 0xc0,
        0x75, 0xf0,
    ],
    &[1; 32],
);

const MERGE_RANK_SIGNATURE: Signature = Signature::new(
    "rank_merge_helper_1412dd790",
    &[
        0x48, 0x8b, 0x05, 0, 0, 0, 0, 0x4c, 0x8d, 0x15, 0, 0, 0, 0, 0xf3, 0x0f, 0x10, 0x15, 0, 0,
        0, 0, 0x4c, 0x8b, 0x40, 0x18, 0x48, 0x63, 0xc1, 0x48, 0x8d, 0x0d,
    ],
    &[
        1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1,
    ],
);

const OVERWRITE_LEN: usize = 14;
const SLOT_SELECTOR_OFFSET: usize = 0x64;
const SLOT_COUNT: usize = 3;
const RANK_THRESHOLD_COUNT: usize = 5;
const MERGE_GRADE_COUNT: usize = 6;
const THRESHOLD_ROWS: [usize; RANK_THRESHOLD_COUNT] = [0x00, 0x0c, 0x18, 0x24, 0x30];
const RESULT_TIME_CALLER_A_RVA: usize = 0x132ad27;
const RESULT_TIME_CALLER_B_RVA: usize = 0x132b8ee;
const RESULT_COUNT_CALLER_RVA: usize = 0x132b917;
const GLOBAL_MERGE_MODE4_DIRECT_CALLER_RVA: usize = 0x132adc1;
const GLOBAL_MERGE_DEFAULT_CALLER_RVA: usize = 0x132aeba;
const GLOBAL_COUNT_CALL_RVA: usize = 0x132ad34;
const RESULT_COUNT_CALL_RVA: usize = 0x132b912;
const RESULT_COUNT_POST_CALL_RVA: usize = 0x132b917;
const RESULT_SPECIAL_CAP_POST_CALL_RVA: usize = 0x132b995;
const RESULT_AGGREGATE_POST_CALL_RVA: usize = 0x132bc7a;
const FIXED_ROOT_RVA: usize = 0x1eba738;
const FIXED_TABLE_OWNER_OFFSET: usize = 0x18;
const FIXED_HELPER_TABLE_OFFSET: usize = 0x28;
const FIXED_SCORE_TABLE_OFFSET: usize = 0x20;
const MERGE_SCORE_OFFSET: usize = 0x1048;
const MERGE_RANK_SCORE_INDEX_RVA: usize = 0x1953390;
const MERGE_GRADE_TARGET_INDEX_RVA: usize = 0x19533a8;

type TimeRankFn = extern "system" fn(usize, f32) -> u8;
type CountRankFn = extern "system" fn(usize, u32, f32) -> u8;
type MergeRankFn = extern "system" fn(i32, i32) -> i32;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static TIME_HOOK: OnceLock<InlineHook> = OnceLock::new();
static COUNT_HOOK: OnceLock<InlineHook> = OnceLock::new();
static MERGE_HOOK: OnceLock<InlineHook> = OnceLock::new();
static TIME_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static COUNT_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static MERGE_TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static PATCH_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static COUNT_THRESHOLD_SHIFT: OnceLock<CountThresholdShiftPatch> = OnceLock::new();
static CALLSITE_COUNT_THRESHOLD_OVERRIDE: OnceLock<[u32; RANK_THRESHOLD_COUNT]> = OnceLock::new();
static CALLSITE_COUNT_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_COUNT_CALL_INSTALLED: AtomicBool = AtomicBool::new(false);
static GLOBAL_COUNT_CALL_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_SPECIAL_CAP_INSTALLED: AtomicBool = AtomicBool::new(false);
static CALLSITE_AGGREGATE_INSTALLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn install(
    host: OwnedHostApi,
    config: RankHelperProbeConfig,
    runtime: RankRuntimeConfig,
) {
    let legacy_shift_requested =
        runtime.shift_count_thresholds && runtime.shift_count_rank_row_ids.is_empty();
    if let Some(thresholds) = runtime.count_threshold_override {
        let _ = CALLSITE_COUNT_THRESHOLD_OVERRIDE.set(thresholds);
    }
    if !config.enabled && !legacy_shift_requested {
        if config.callsite_enabled {
            let _ = HOST.set(host.clone());
            LOG_ENABLED.store(true, Ordering::Relaxed);
            MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
            install_callsite_hooks(&host);
            return;
        }
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    LOG_ENABLED.store(config.enabled, Ordering::Relaxed);
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
    if legacy_shift_requested {
        let _ = COUNT_THRESHOLD_SHIFT.set(CountThresholdShiftPatch {
            source_prefix: runtime.shift_count_source_prefix,
            row_offset: runtime.shift_count_row_offset,
            inserted_first: runtime.shift_count_inserted_first,
        });
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "rank_runtime count threshold shift enabled row_offset={} source_prefix={:?} inserted_first={}",
                format_optional_offset(runtime.shift_count_row_offset),
                runtime.shift_count_source_prefix,
                runtime.shift_count_inserted_first
            ),
        );
    }

    if config.enabled {
        install_time_hook(&host);
    }

    if config.count_enabled {
        install_count_hook(&host);
    } else if legacy_shift_requested {
        let _ = host.log().write(
            PLUGIN_ID,
            "rank_runtime count threshold shift requested but count hook is disabled; no rank count patch installed",
        );
    }

    if config.merge_enabled {
        install_merge_hook(&host);
    }

    if config.callsite_enabled {
        install_callsite_hooks(&host);
    }
}

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

const CALLSITE_COUNT_KIND: u32 = 2;
const CALLSITE_SPECIAL_CAP_KIND: u32 = 3;
const CALLSITE_AGGREGATE_KIND: u32 = 4;
const CALLSITE_COUNT_CALL_KIND: u32 = 5;
const CALLSITE_GLOBAL_COUNT_CALL_KIND: u32 = 6;

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

fn install_callsite_hooks(host: &OwnedHostApi) {
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

fn install_time_hook(host: &OwnedHostApi) {
    if TIME_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_probe time hook already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(TIME_RANK_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder
                    .install_abs_jump_with_return_address(time_rank_detour as *const () as usize)?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            TIME_TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let trampoline = hook.trampoline;
            let _ = TIME_HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_helper_probe time hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_probe time hook install failed: {error}"),
            );
        }
    }
}

fn install_count_hook(host: &OwnedHostApi) {
    if COUNT_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_probe count hook already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(COUNT_RANK_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump_with_return_address(
                    count_rank_detour as *const () as usize,
                )?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            COUNT_TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let trampoline = hook.trampoline;
            let _ = COUNT_HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_helper_probe count hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_probe count hook install failed: {error}"),
            );
        }
    }
}

fn install_merge_hook(host: &OwnedHostApi) {
    if MERGE_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_probe merge hook already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(MERGE_RANK_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump_with_return_address(
                    merge_rank_detour as *const () as usize,
                )?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            MERGE_TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let trampoline = hook.trampoline;
            let _ = MERGE_HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_helper_probe merge hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_probe merge hook install failed: {error}"),
            );
        }
    }
}

extern "system" fn time_rank_detour(row: usize, value: f32, _unused: usize, caller: usize) -> u8 {
    let original = TIME_TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return 0;
    }

    let original: TimeRankFn = unsafe { mem::transmute(original) };
    let result = original(row, value);

    let _ = panic::catch_unwind(|| {
        log_time_rank(caller, row, value, result);
    });

    result
}

extern "system" fn count_rank_detour(row: usize, value: u32, divisor: f32, caller: usize) -> u8 {
    let original = COUNT_TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return 0;
    }

    let _ = panic::catch_unwind(|| {
        apply_count_threshold_shift(row);
    });

    let original: CountRankFn = unsafe { mem::transmute(original) };
    let result = original(row, value, divisor);

    let _ = panic::catch_unwind(|| {
        log_count_rank(caller, row, value, divisor, result);
    });

    result
}

extern "system" fn merge_rank_detour(
    left_rank: i32,
    right_rank: i32,
    _unused: usize,
    caller: usize,
) -> i32 {
    let original = MERGE_TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return 0;
    }

    let original: MergeRankFn = unsafe { mem::transmute(original) };
    let result = original(left_rank, right_rank);

    let _ = panic::catch_unwind(|| {
        log_merge_rank(caller, left_rank, right_rank, result);
    });

    result
}

fn log_time_rank(caller: usize, row: usize, value: f32, result: u8) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(snapshot) = RankHelperRow::read(row, 0) else {
        return;
    };
    let caller_label = caller_label(caller);
    let row_offset = rank_row_offset(row);
    let payload = RankHelperCallSignal::time(caller, caller_label, row, value, result, snapshot);
    record_rank_helper(
        format!(
        "rank_helper_probe kind=time caller={} caller_label={} row=0x{row:x} row_offset={} slot={} value={value:.3} thresholds=[{}] all_slots=[{}] result={}({}) selectors=[{}]",
        payload.caller,
        caller_label,
        format_optional_offset(row_offset),
        snapshot.slot,
        snapshot.thresholds_csv(),
        snapshot.all_thresholds_csv(),
        result,
        result_label(result),
        snapshot.selectors_csv(),
    ),
        &payload,
    );
}

fn log_count_rank(caller: usize, row: usize, value: u32, divisor: f32, result: u8) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(snapshot) = RankHelperRow::read(row, 1) else {
        return;
    };
    let normalized = if divisor == 0.0 {
        0
    } else {
        ((value as f32) / divisor) as u32
    };
    let caller_label = caller_label(caller);
    let payload = RankHelperCallSignal::count(
        caller,
        caller_label,
        row,
        value,
        divisor,
        normalized,
        result,
        snapshot,
    );
    let row_offset = rank_row_offset(row);
    record_rank_helper(
        format!(
        "rank_helper_probe kind=count caller={} caller_label={} row=0x{row:x} row_offset={} slot={} value={} divisor={divisor:.3} normalized={} thresholds=[{}] all_slots=[{}] per_value=[{}] result={}({}) selectors=[{}]",
        payload.caller,
        caller_label,
        format_optional_offset(row_offset),
        snapshot.slot,
        value,
        normalized,
        snapshot.thresholds_csv(),
        snapshot.all_thresholds_csv(),
        snapshot.threshold_ratios_csv(value),
        result,
        result_label(result),
        snapshot.selectors_csv(),
    ),
        &payload,
    );
}

fn log_merge_rank(caller: usize, left_rank: i32, right_rank: i32, result: i32) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let caller_label = caller_label(caller);
    let score = MergeScoreSnapshot::read(left_rank, right_rank);
    let payload = RankMergeCallSignal {
        kind: "merge",
        caller: format_caller(caller),
        caller_rva: caller_rva(caller),
        caller_label,
        left_rank,
        left_label: result_label_i32(left_rank),
        right_rank,
        right_label: result_label_i32(right_rank),
        result,
        result_label: result_label_i32(result),
        score,
    };
    let score_text = payload
        .score
        .as_ref()
        .map(|score| {
            format!(
                " left_score={} right_score={} combined_score={} grade_targets=[{}]",
                score.left_score,
                score.right_score,
                score.combined_score,
                csv(score.grade_targets)
            )
        })
        .unwrap_or_default();

    record_rank_helper(
        format!(
            "rank_helper_probe kind=merge caller={} caller_label={} left={}({}) right={}({}) result={}({}){}",
            payload.caller,
            caller_label,
            left_rank,
            payload.left_label,
            right_rank,
            payload.right_label,
            result,
            payload.result_label,
            score_text,
        ),
        &payload,
    );
}

fn apply_count_threshold_shift(row: usize) {
    let Some(patch) = COUNT_THRESHOLD_SHIFT.get() else {
        return;
    };
    let Some(snapshot) = RankHelperRow::read(row, 1) else {
        return;
    };
    if !snapshot.matches_prefix(patch.source_prefix) {
        return;
    }
    let row_offset = rank_row_offset(row);
    if patch.row_offset != row_offset {
        if PATCH_LOG_COUNT.fetch_add(1, Ordering::Relaxed) < 16 {
            if let Some(host) = HOST.get() {
                let _ = host.log().write(
                    PLUGIN_ID,
                    format!(
                        "rank_runtime count threshold candidate row=0x{row:x} row_offset={} expected_offset={} thresholds=[{}]; not patched",
                        format_optional_offset(row_offset),
                        format_optional_offset(patch.row_offset),
                        csv(snapshot.thresholds),
                    ),
                );
            }
        }
        return;
    }

    let patched = snapshot.shifted_thresholds(patch.inserted_first);
    unsafe {
        write_thresholds(row, snapshot.slot, patched);
    }
    if PATCH_LOG_COUNT.fetch_add(1, Ordering::Relaxed) < 16 {
        if let Some(host) = HOST.get() {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_runtime count thresholds shifted row=0x{row:x} row_offset={} slot={} old=[{}] new=[{}]",
                    format_optional_offset(row_offset),
                    snapshot.slot,
                    csv(snapshot.thresholds),
                    csv(patched),
                ),
            );
        }
    }
}

fn format_caller(caller: usize) -> String {
    let base = module_base();
    if caller >= base {
        format!("game+0x{:x}", caller - base)
    } else {
        format!("0x{caller:x}")
    }
}

fn caller_rva(caller: usize) -> Option<usize> {
    let base = module_base();
    (caller >= base).then_some(caller - base)
}

fn caller_label(caller: usize) -> &'static str {
    match caller_rva(caller) {
        Some(RESULT_TIME_CALLER_A_RVA) => "result_time_candidate_a",
        Some(RESULT_TIME_CALLER_B_RVA) => "result_time_candidate_b",
        Some(RESULT_COUNT_CALLER_RVA) => "result_defeated_count_candidate",
        Some(GLOBAL_MERGE_MODE4_DIRECT_CALLER_RVA) => "global_mode4_direct_merge",
        Some(GLOBAL_MERGE_DEFAULT_CALLER_RVA) => "global_default_merge",
        _ => "unknown",
    }
}

fn result_label(result: u8) -> &'static str {
    result_label_i32(result.into())
}

fn result_label_i32(result: i32) -> &'static str {
    match result {
        0 => "D",
        1 => "C",
        2 => "B",
        3 => "A",
        4 => "S",
        5 => "S+",
        _ => "unknown",
    }
}

fn format_optional_offset(offset: Option<usize>) -> String {
    offset
        .map(|offset| format!("0x{offset:x}"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn rank_row_offset(row: usize) -> Option<usize> {
    let fixed_helper_table = read_fixed_helper_table().ok()?;
    row.checked_sub(fixed_helper_table)
        .filter(|offset| *offset <= 0x10_0000)
}

fn read_fixed_helper_table() -> Result<usize, String> {
    let base = module_base();
    let root = read_process_usize(base + FIXED_ROOT_RVA)?;
    let owner = read_process_usize(root + FIXED_TABLE_OWNER_OFFSET)?;
    read_process_usize(owner + FIXED_HELPER_TABLE_OFFSET)
}

fn read_fixed_score_table() -> Result<usize, String> {
    let base = module_base();
    let root = read_process_usize(base + FIXED_ROOT_RVA)?;
    let owner = read_process_usize(root + FIXED_TABLE_OWNER_OFFSET)?;
    read_process_usize(owner + FIXED_SCORE_TABLE_OFFSET)
}

fn read_process_usize(address: usize) -> Result<usize, String> {
    let bytes = read_process_bytes::<8>(address)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn read_process_i32(address: usize) -> Result<i32, String> {
    let bytes = read_process_bytes::<4>(address)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_process_bytes<const N: usize>(address: usize) -> Result<[u8; N], String> {
    let Some(host) = HOST.get() else {
        return Err("host unavailable".to_string());
    };
    let mut bytes = [0u8; N];
    host.memory()
        .read(address, &mut bytes)
        .map_err(|error| format!("read failed address=0x{address:x}: {error}"))?;
    Ok(bytes)
}

fn record_rank_helper<T: Serialize>(message: String, payload: &T) {
    let index = LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    if let Some(host) = HOST.get() {
        let _ = host.log().write(PLUGIN_ID, message);
        signals::emit_json(host, signals::RANK_HELPER_CALL, payload);
    }
}

#[derive(Clone, Debug, Serialize)]
struct RankHelperCallSignal {
    kind: &'static str,
    caller: String,
    caller_rva: Option<usize>,
    caller_label: &'static str,
    row: usize,
    row_offset: Option<usize>,
    slot: usize,
    selectors: [u16; SLOT_COUNT],
    thresholds: [u32; RANK_THRESHOLD_COUNT],
    all_thresholds: [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
    value_f32: Option<f32>,
    value_u32: Option<u32>,
    divisor: Option<f32>,
    normalized: Option<u32>,
    result: u8,
    result_label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct RankMergeCallSignal {
    kind: &'static str,
    caller: String,
    caller_rva: Option<usize>,
    caller_label: &'static str,
    left_rank: i32,
    left_label: &'static str,
    right_rank: i32,
    right_label: &'static str,
    result: i32,
    result_label: &'static str,
    score: Option<MergeScoreSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
struct MergeScoreSnapshot {
    left_score_index: i32,
    right_score_index: i32,
    left_score: u32,
    right_score: u32,
    combined_score: u32,
    grade_score_indexes: [i32; MERGE_GRADE_COUNT],
    grade_targets: [u32; MERGE_GRADE_COUNT],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CountThresholdShiftPatch {
    source_prefix: [u32; 3],
    row_offset: Option<usize>,
    inserted_first: u32,
}

impl RankHelperCallSignal {
    fn time(
        caller: usize,
        caller_label: &'static str,
        row: usize,
        value: f32,
        result: u8,
        snapshot: RankHelperRow,
    ) -> Self {
        Self {
            kind: "time",
            caller: format_caller(caller),
            caller_rva: caller_rva(caller),
            caller_label,
            row,
            row_offset: rank_row_offset(row),
            slot: snapshot.slot,
            selectors: snapshot.selectors,
            thresholds: snapshot.thresholds,
            all_thresholds: snapshot.all_thresholds,
            value_f32: Some(value),
            value_u32: None,
            divisor: None,
            normalized: None,
            result,
            result_label: result_label(result),
        }
    }

    fn count(
        caller: usize,
        caller_label: &'static str,
        row: usize,
        value: u32,
        divisor: f32,
        normalized: u32,
        result: u8,
        snapshot: RankHelperRow,
    ) -> Self {
        Self {
            kind: "count",
            caller: format_caller(caller),
            caller_rva: caller_rva(caller),
            caller_label,
            row,
            row_offset: rank_row_offset(row),
            slot: snapshot.slot,
            selectors: snapshot.selectors,
            thresholds: snapshot.thresholds,
            all_thresholds: snapshot.all_thresholds,
            value_f32: None,
            value_u32: Some(value),
            divisor: Some(divisor),
            normalized: Some(normalized),
            result,
            result_label: result_label(result),
        }
    }
}

impl MergeScoreSnapshot {
    fn read(left_rank: i32, right_rank: i32) -> Option<Self> {
        let base = module_base();
        let fixed_score_table = read_fixed_score_table().ok()?;
        let left_score_index = read_rank_score_index(base, left_rank).ok()?;
        let right_score_index = read_rank_score_index(base, right_rank).ok()?;
        let left_score = read_scaled_score(fixed_score_table, left_score_index).ok()?;
        let right_score = read_scaled_score(fixed_score_table, right_score_index).ok()?;
        let grade_score_indexes =
            std::array::from_fn(|grade| read_grade_target_index(base, grade).unwrap_or_default());
        let grade_targets = std::array::from_fn(|grade| {
            read_scaled_score(fixed_score_table, grade_score_indexes[grade]).unwrap_or_default()
        });

        Some(Self {
            left_score_index,
            right_score_index,
            left_score,
            right_score,
            combined_score: left_score + right_score,
            grade_score_indexes,
            grade_targets,
        })
    }
}

fn read_rank_score_index(base: usize, rank: i32) -> Result<i32, String> {
    if !(0..=6).contains(&rank) {
        return Err(format!("rank out of score index range: {rank}"));
    }
    read_process_i32(base + MERGE_RANK_SCORE_INDEX_RVA + rank as usize * size_of::<i32>())
}

fn read_grade_target_index(base: usize, grade: usize) -> Result<i32, String> {
    read_process_i32(base + MERGE_GRADE_TARGET_INDEX_RVA + grade * size_of::<i32>())
}

fn read_scaled_score(fixed_score_table: usize, score_index: i32) -> Result<u32, String> {
    if score_index < 0 {
        return Err(format!("negative score index: {score_index}"));
    }
    let raw = read_process_i32(
        fixed_score_table + MERGE_SCORE_OFFSET + score_index as usize * size_of::<i32>(),
    )?;
    Ok((raw as f32 * 0.001) as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RankHelperRow {
    slot: usize,
    selectors: [u16; SLOT_COUNT],
    thresholds: [u32; RANK_THRESHOLD_COUNT],
    all_thresholds: [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
}

impl RankHelperRow {
    fn read(row: usize, selector: u16) -> Option<Self> {
        if row == 0 {
            return None;
        }

        let selectors = unsafe { read_selectors(row) };
        let slot = selectors
            .iter()
            .position(|candidate| *candidate == selector)?;
        let all_thresholds = unsafe { read_all_thresholds(row) };
        let thresholds = all_thresholds[slot];
        Some(Self {
            slot,
            selectors,
            thresholds,
            all_thresholds,
        })
    }

    fn thresholds_csv(self) -> String {
        csv(self.thresholds)
    }

    fn threshold_ratios_csv(self, value: u32) -> String {
        if value == 0 {
            return "inf,inf,inf,inf,inf".to_string();
        }

        self.thresholds
            .map(|threshold| format!("{:.3}", threshold as f32 / value as f32))
            .join(",")
    }

    fn all_thresholds_csv(self) -> String {
        self.all_thresholds
            .iter()
            .enumerate()
            .map(|(slot, thresholds)| format!("s{slot}:{}", csv(*thresholds)))
            .collect::<Vec<_>>()
            .join(";")
    }

    fn selectors_csv(self) -> String {
        csv(self.selectors)
    }

    fn matches_prefix(self, prefix: [u32; 3]) -> bool {
        self.thresholds[..prefix.len()] == prefix
    }

    fn shifted_thresholds(self, inserted_first: u32) -> [u32; RANK_THRESHOLD_COUNT] {
        [
            inserted_first,
            self.thresholds[0],
            self.thresholds[1],
            self.thresholds[2],
            self.thresholds[3],
        ]
    }
}

fn csv<const N: usize, T: ToString>(values: [T; N]) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

unsafe fn read_selectors(row: usize) -> [u16; SLOT_COUNT] {
    let ptr = row as *const u8;
    [
        read_u16(ptr, SLOT_SELECTOR_OFFSET),
        read_u16(ptr, SLOT_SELECTOR_OFFSET + 2),
        read_u16(ptr, SLOT_SELECTOR_OFFSET + 4),
    ]
}

unsafe fn read_thresholds(row: usize, slot: usize) -> [u32; RANK_THRESHOLD_COUNT] {
    let ptr = row as *const u8;
    THRESHOLD_ROWS.map(|offset| read_u32(ptr, offset + slot * 4))
}

unsafe fn read_all_thresholds(row: usize) -> [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT] {
    [
        read_thresholds(row, 0),
        read_thresholds(row, 1),
        read_thresholds(row, 2),
    ]
}

unsafe fn read_u16(ptr: *const u8, offset: usize) -> u16 {
    (ptr.add(offset) as *const u16).read_unaligned()
}

unsafe fn read_u32(ptr: *const u8, offset: usize) -> u32 {
    (ptr.add(offset) as *const u32).read_unaligned()
}

unsafe fn write_thresholds(row: usize, slot: usize, thresholds: [u32; RANK_THRESHOLD_COUNT]) {
    let ptr = row as *mut u8;
    for (offset, value) in THRESHOLD_ROWS.into_iter().zip(thresholds) {
        (ptr.add(offset + slot * size_of::<u32>()) as *mut u32).write_unaligned(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rank_helper_row_for_selector_slot() {
        let mut row = [0u8; 0x70];
        write_u32(&mut row, 0x00, 100);
        write_u32(&mut row, 0x0c, 200);
        write_u32(&mut row, 0x18, 300);
        write_u32(&mut row, 0x24, 400);
        write_u32(&mut row, 0x30, 500);
        write_u16(&mut row, 0x64, 0);
        write_u16(&mut row, 0x66, 1);
        write_u16(&mut row, 0x68, 99);

        let snapshot = RankHelperRow::read(row.as_ptr() as usize, 0).expect("row");

        assert_eq!(snapshot.slot, 0);
        assert_eq!(snapshot.selectors, [0, 1, 99]);
        assert_eq!(snapshot.thresholds, [100, 200, 300, 400, 500]);
        assert_eq!(
            snapshot.all_thresholds_csv(),
            "s0:100,200,300,400,500;s1:0,0,0,0,0;s2:0,0,0,0,0"
        );
    }

    #[test]
    fn ignores_unknown_selector() {
        let mut row = [0u8; 0x70];
        write_u16(&mut row, 0x64, 0);
        write_u16(&mut row, 0x66, 1);
        write_u16(&mut row, 0x68, 99);

        assert_eq!(RankHelperRow::read(row.as_ptr() as usize, 2), None);
    }

    #[test]
    fn labels_known_rank_results() {
        assert_eq!(result_label(0), "D");
        assert_eq!(result_label(3), "A");
        assert_eq!(result_label(5), "S+");
        assert_eq!(result_label(9), "unknown");
    }

    #[test]
    fn shifts_count_thresholds_one_slot_right() {
        let row = RankHelperRow {
            slot: 1,
            selectors: [0, 1, 99],
            thresholds: [60_000, 60_000, 48_000, 42_000, 30_000],
            all_thresholds: [[0; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
        };

        assert!(row.matches_prefix([60_000, 60_000, 48_000]));
        assert_eq!(
            row.shifted_thresholds(72_000),
            [72_000, 60_000, 60_000, 48_000, 42_000]
        );
    }

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

    fn write_u16(row: &mut [u8], offset: usize, value: u16) {
        row[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(row: &mut [u8], offset: usize, value: u32) {
        row[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
