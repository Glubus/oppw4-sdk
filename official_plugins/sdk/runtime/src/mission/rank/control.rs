use std::{
    ffi::{c_char, c_void},
    sync::OnceLock,
};

use plugin_sdk::{
    OwnedHostApi, RankCapEffect, RankConditionExpr, RankSlot, RANK_OVERRIDE_COUNT_THRESHOLDS,
    RANK_SET_CAP, RANK_SHIFT_COUNT_THRESHOLDS,
};
use serde::Deserialize;

use crate::{config::RankRuntimeConfig, runtime::probe::PLUGIN_ID};

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn install(host: OwnedHostApi) {
    let _ = HOST.set(host.clone());
    unsafe {
        subscribe(&host, RANK_SET_CAP);
        subscribe(&host, RANK_SHIFT_COUNT_THRESHOLDS);
        subscribe(&host, RANK_OVERRIDE_COUNT_THRESHOLDS);
    }
}

unsafe fn subscribe(host: &OwnedHostApi, signal: &str) {
    if let Err(error) =
        host.signals()
            .subscribe_bytes(signal, std::ptr::null_mut(), rank_command_callback)
    {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("rank_runtime subscribe failed signal={signal}: {error}"),
        );
    }
}

unsafe extern "system" fn rank_command_callback(
    _subscriber_context: *mut c_void,
    signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    let Some(host) = HOST.get() else {
        return -1;
    };
    let signal = unsafe { std::ffi::CStr::from_ptr(signal_utf8) }.to_string_lossy();
    let bytes = if payload_len == 0 {
        &[]
    } else if payload.is_null() {
        return -2;
    } else {
        unsafe { std::slice::from_raw_parts(payload, payload_len) }
    };
    match handle_command(host, &signal, bytes) {
        Ok(()) => 0,
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("rank_runtime command failed signal={signal}: {error}"),
            );
            -3
        }
    }
}

fn handle_command(host: &OwnedHostApi, signal: &str, payload: &[u8]) -> Result<(), String> {
    match signal {
        RANK_SET_CAP => apply_rank_cap(host, read_json(payload)?),
        RANK_SHIFT_COUNT_THRESHOLDS => apply_shift(host, read_json(payload)?),
        RANK_OVERRIDE_COUNT_THRESHOLDS => apply_override(host, read_json(payload)?),
        _ => Ok(()),
    }
}

fn apply_rank_cap(host: &OwnedHostApi, rule: RuntimeRankCapRule) -> Result<(), String> {
    match rule.effect {
        RankCapEffect::Enable
            if rule
                .slots
                .iter()
                .any(|slot| matches!(slot.as_str(), "s" | "s_plus"))
                && rule.condition.is_none() =>
        {
            super::easy_cap::set_easy_s_rankable(host, rule.enabled);
            Ok(())
        }
        RankCapEffect::KeepDefault => Ok(()),
        effect => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "rank_runtime unsupported cap rule slots={:?} effect={effect:?} condition={:?}",
                    rule.slots, rule.condition
                ),
            );
            Ok(())
        }
    }
}

fn apply_shift(host: &OwnedHostApi, request: RuntimeCountThresholdShift) -> Result<(), String> {
    if request.rank_row_ids.is_empty() {
        return Err("rank_row_ids is empty".to_string());
    }
    super::threshold_patch::install(
        host.clone(),
        RankRuntimeConfig {
            shift_count_thresholds: true,
            shift_count_rank_row_ids: request.rank_row_ids,
            shift_count_source_prefix: request.source_prefix,
            shift_count_inserted_first: request.inserted_first,
            shift_count_inserted_second: request.inserted_second,
            ..RankRuntimeConfig::default()
        },
    );
    Ok(())
}

fn apply_override(
    host: &OwnedHostApi,
    request: RuntimeCountThresholdOverride,
) -> Result<(), String> {
    if request.rank_row_ids.is_empty() {
        return Err("rank_row_ids is empty".to_string());
    }
    super::threshold_patch::install(
        host.clone(),
        RankRuntimeConfig {
            shift_count_thresholds: true,
            shift_count_rank_row_ids: request.rank_row_ids,
            shift_count_source_prefix: request.source_prefix,
            count_threshold_override: Some(request.thresholds),
            ..RankRuntimeConfig::default()
        },
    );
    Ok(())
}

fn read_json<'a, T>(payload: &'a [u8]) -> Result<T, String>
where
    T: Deserialize<'a>,
{
    serde_json::from_slice(payload).map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Deserialize)]
struct RuntimeRankCapRule {
    slots: Vec<RankSlot>,
    condition: RankConditionExpr,
    effect: RankCapEffect,
    enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct RuntimeCountThresholdShift {
    rank_row_ids: Vec<u16>,
    source_prefix: [u32; 3],
    inserted_first: u32,
    inserted_second: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
struct RuntimeCountThresholdOverride {
    rank_row_ids: Vec<u16>,
    source_prefix: [u32; 3],
    thresholds: [u32; 5],
}

#[cfg(test)]
mod tests {
    use plugin_sdk::{CountThresholdOverride, CountThresholdShift, RankCapRule, RankCondition};

    use super::*;

    #[test]
    fn rank_cap_payload_uses_public_names() {
        let payload = serde_json::to_vec(&RankCapRule::enable_slots([4_u32, 5])).expect("json");
        let rule: RuntimeRankCapRule = read_json(&payload).expect("rule");

        assert_eq!(rule.slots, [RankSlot::s(), RankSlot::s_plus()]);
        assert_eq!(rule.condition, RankConditionExpr::None);
        assert_eq!(rule.effect, RankCapEffect::Enable);
        assert!(rule.enabled);
    }

    #[test]
    fn cap_rule_can_target_a_slot_with_generic_conditions() {
        let payload = serde_json::to_vec(&RankCapRule::enable_slots([4_u32, 5]).all([
            RankCondition::active_character("zoro"),
            RankCondition::flag("crew.elbaph", true),
        ]))
        .expect("json");
        let rule: RuntimeRankCapRule = read_json(&payload).expect("rule");

        assert_eq!(rule.slots, [RankSlot::s(), RankSlot::s_plus()]);
        assert_eq!(
            rule.condition,
            RankConditionExpr::all([
                RankCondition::active_character("zoro"),
                RankCondition::flag("crew.elbaph", true)
            ])
        );
        assert_eq!(rule.effect, RankCapEffect::Enable);
        assert!(rule.enabled);
    }

    #[test]
    fn cap_rule_can_disable_any_rank_name() {
        let payload = serde_json::to_vec(&RankCapRule::disable().slots(["d"])).expect("json");
        let rule: RuntimeRankCapRule = read_json(&payload).expect("rule");

        assert_eq!(rule.slots, [RankSlot::d()]);
        assert_eq!(rule.effect, RankCapEffect::Disable);
        assert_eq!(rule.condition, RankConditionExpr::None);
    }

    #[test]
    fn threshold_payload_round_trips() {
        let payload = serde_json::to_vec(&CountThresholdShift::new(vec![35])).expect("json");
        let request: RuntimeCountThresholdShift = read_json(&payload).expect("request");

        assert_eq!(request.rank_row_ids, [35]);
        assert_eq!(
            request.source_prefix,
            CountThresholdShift::DEFAULT_SOURCE_PREFIX
        );
    }

    #[test]
    fn override_payload_round_trips() {
        let payload = serde_json::to_vec(&CountThresholdOverride::new(vec![35], [1, 2, 3, 4, 5]))
            .expect("json");
        let request: RuntimeCountThresholdOverride = read_json(&payload).expect("request");

        assert_eq!(request.rank_row_ids, [35]);
        assert_eq!(request.thresholds, [1, 2, 3, 4, 5]);
    }
}
