use std::{
    mem, panic,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use crate::{
    config::{RankHelperHooksConfig, RankRuntimeConfig},
    runtime::{probe::PLUGIN_ID, signals},
};

mod callsite;
mod labels;
mod memory;
mod payload;
mod row;

use callsite::{install_callsite_hooks, set_count_threshold_override};
use labels::{
    caller_label, caller_rva, format_caller, format_optional_offset, result_label, result_label_i32,
};
use memory::rank_row_offset;
use payload::{MergeScoreSnapshot, RankHelperCallSignal, RankMergeCallSignal};
use row::{csv, write_thresholds, RankHelperRow};

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

pub(crate) fn install(
    host: OwnedHostApi,
    config: RankHelperHooksConfig,
    runtime: RankRuntimeConfig,
) {
    let legacy_shift_requested =
        runtime.shift_count_thresholds && runtime.shift_count_rank_row_ids.is_empty();
    if let Some(thresholds) = runtime.count_threshold_override {
        set_count_threshold_override(thresholds);
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
            .write(PLUGIN_ID, "rank_helper_hooks disabled by config");
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

    install_time_hook(&host);

    install_count_hook(&host);
    if !config.count_enabled && legacy_shift_requested {
        let _ = host.log().write(
            PLUGIN_ID,
            "rank_runtime count threshold shift requested while count diagnostics are disabled; installing count hook for runtime patch compatibility",
        );
    }

    if config.merge_enabled {
        install_merge_hook(&host);
    }

    if config.callsite_enabled {
        install_callsite_hooks(&host);
    }
}

fn install_time_hook(host: &OwnedHostApi) {
    if TIME_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_hooks time hook already installed");
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
                    "rank_helper_hooks time hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_hooks time hook install failed: {error}"),
            );
        }
    }
}

fn install_count_hook(host: &OwnedHostApi) {
    if COUNT_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_hooks count hook already installed");
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
                    "rank_helper_hooks count hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_hooks count hook install failed: {error}"),
            );
        }
    }
}

fn install_merge_hook(host: &OwnedHostApi) {
    if MERGE_HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "rank_helper_hooks merge hook already installed");
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
                    "rank_helper_hooks merge hook installed site=0x{site:x} trampoline=0x{trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_helper_hooks merge hook install failed: {error}"),
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
    let original_result = original(row, value);

    let override_result =
        panic::catch_unwind(|| query_time_rank(caller, row, value, original_result))
            .ok()
            .flatten();

    let _ = panic::catch_unwind(|| {
        log_time_rank(
            caller,
            row,
            value,
            override_result.unwrap_or(original_result),
        );
    });

    override_result.unwrap_or(original_result)
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
    let original_result = original(row, value, divisor);

    let override_result =
        panic::catch_unwind(|| query_count_rank(caller, row, value, divisor, original_result))
            .ok()
            .flatten();

    let _ = panic::catch_unwind(|| {
        log_count_rank(
            caller,
            row,
            value,
            divisor,
            override_result.unwrap_or(original_result),
        );
    });

    override_result.unwrap_or(original_result)
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
        "rank_helper_hooks kind=time caller={} caller_label={} row=0x{row:x} row_offset={} slot={} value={value:.3} thresholds=[{}] all_slots=[{}] result={}({}) selectors=[{}]",
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
        "rank_helper_hooks kind=count caller={} caller_label={} row=0x{row:x} row_offset={} slot={} value={} divisor={divisor:.3} normalized={} thresholds=[{}] all_slots=[{}] per_value=[{}] result={}({}) selectors=[{}]",
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
            "rank_helper_hooks kind=merge caller={} caller_label={} left={}({}) right={}({}) result={}({}){}",
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

fn query_time_rank(caller: usize, row: usize, value: f32, original_result: u8) -> Option<u8> {
    let host = HOST.get()?;
    if !host.signals().has_listeners(signals::RANK_CALC_TIME) {
        return None;
    }
    let snapshot = RankHelperRow::read(row, 0)?;
    let caller_label = caller_label(caller);
    let payload =
        RankHelperCallSignal::time(caller, caller_label, row, value, original_result, snapshot);
    query_rank_override(host, signals::RANK_CALC_TIME, &payload)
}

fn query_count_rank(
    caller: usize,
    row: usize,
    value: u32,
    divisor: f32,
    original_result: u8,
) -> Option<u8> {
    let host = HOST.get()?;
    if !host.signals().has_listeners(signals::RANK_CALC_COUNT) {
        return None;
    }
    let snapshot = RankHelperRow::read(row, 1)?;
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
        original_result,
        snapshot,
    );
    query_rank_override(host, signals::RANK_CALC_COUNT, &payload)
}

fn query_rank_override<T: Serialize>(host: &OwnedHostApi, signal: &str, payload: &T) -> Option<u8> {
    let bytes = serde_json::to_vec(payload).ok()?;
    let json = match host.signals().query_json(signal, &bytes) {
        Ok(Some(json)) => json,
        Ok(None) => return None,
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank calc query failed signal={signal} error={error}"),
            );
            return None;
        }
    };
    match serde_json::from_str::<String>(&json) {
        Ok(rank) => parse_rank_override(&rank),
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank calc query returned invalid json signal={signal} error={error}"),
            );
            None
        }
    }
}

fn parse_rank_override(rank: &str) -> Option<u8> {
    match rank.trim().to_ascii_uppercase().as_str() {
        "S+" => Some(5),
        "S" => Some(4),
        "A" => Some(3),
        "B" => Some(2),
        "C" => Some(1),
        "D" => Some(0),
        _ => None,
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CountThresholdShiftPatch {
    source_prefix: [u32; 3],
    row_offset: Option<usize>,
    inserted_first: u32,
}
