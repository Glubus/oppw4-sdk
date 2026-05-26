use plugin_sdk::OwnedHostApi;

use crate::{
    config::RankRuntimeConfig,
    runtime::{
        core::{
            events::RuntimeMutation,
            rank::{RankMutation, RankValue},
        },
        probe::PLUGIN_ID,
    },
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RankApplyReport {
    pub(crate) applied: usize,
    pub(crate) skipped: usize,
}

pub(crate) fn apply_rank_mutations(
    host: &OwnedHostApi,
    mutations: &[RuntimeMutation],
) -> RankApplyReport {
    let mut report = RankApplyReport::default();
    for mutation in mutations {
        let RuntimeMutation::Rank(mutation) = mutation else {
            continue;
        };
        match mutation_action(mutation) {
            RankMutationAction::EasyCap { enabled } => {
                super::easy_cap::set_easy_s_rankable(host, enabled);
                report.applied += 1;
            }
            RankMutationAction::ShiftCountThresholds {
                rank_row_ids,
                source_prefix,
                inserted_first,
                inserted_second,
            } => {
                super::threshold_patch::install(
                    host.clone(),
                    RankRuntimeConfig {
                        shift_count_thresholds: true,
                        shift_count_rank_row_ids: rank_row_ids,
                        shift_count_source_prefix: source_prefix,
                        shift_count_inserted_first: inserted_first,
                        shift_count_inserted_second: inserted_second,
                        ..RankRuntimeConfig::default()
                    },
                );
                report.applied += 1;
            }
            RankMutationAction::OverrideCountThresholds {
                rank_row_ids,
                source_prefix,
                thresholds,
            } => {
                super::threshold_patch::install(
                    host.clone(),
                    RankRuntimeConfig {
                        shift_count_thresholds: true,
                        shift_count_rank_row_ids: rank_row_ids,
                        shift_count_source_prefix: source_prefix,
                        count_threshold_override: Some(thresholds),
                        ..RankRuntimeConfig::default()
                    },
                );
                report.applied += 1;
            }
            RankMutationAction::Unsupported(reason) => {
                report.skipped += 1;
                let _ = host.log().write(
                    PLUGIN_ID,
                    format!("rank_runtime mutation skipped: {reason}"),
                );
            }
        }
    }
    report
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RankMutationAction {
    EasyCap {
        enabled: bool,
    },
    ShiftCountThresholds {
        rank_row_ids: Vec<u16>,
        source_prefix: [u32; 3],
        inserted_first: u32,
        inserted_second: Option<u32>,
    },
    OverrideCountThresholds {
        rank_row_ids: Vec<u16>,
        source_prefix: [u32; 3],
        thresholds: [u32; 5],
    },
    Unsupported(String),
}

fn mutation_action(mutation: &RankMutation) -> RankMutationAction {
    match mutation {
        RankMutation::SetCap { rank, enabled }
            if matches!(rank, RankValue::S | RankValue::SPlus) =>
        {
            RankMutationAction::EasyCap { enabled: *enabled }
        }
        RankMutation::SetCap { rank, enabled } => RankMutationAction::Unsupported(format!(
            "set_cap rank={rank} enabled={enabled} only S/S+ cap is confirmed"
        )),
        RankMutation::ShiftCountThresholds {
            rank_row_ids,
            source_prefix,
            inserted_first,
            inserted_second,
        } if !rank_row_ids.is_empty() => RankMutationAction::ShiftCountThresholds {
            rank_row_ids: rank_row_ids.clone(),
            source_prefix: *source_prefix,
            inserted_first: *inserted_first,
            inserted_second: *inserted_second,
        },
        RankMutation::ShiftCountThresholds { .. } => {
            RankMutationAction::Unsupported("shift_count_thresholds row ids are empty".to_string())
        }
        RankMutation::OverrideCountThresholds {
            rank_row_ids,
            source_prefix,
            thresholds,
        } if !rank_row_ids.is_empty() => RankMutationAction::OverrideCountThresholds {
            rank_row_ids: rank_row_ids.clone(),
            source_prefix: *source_prefix,
            thresholds: *thresholds,
        },
        RankMutation::OverrideCountThresholds { .. } => RankMutationAction::Unsupported(
            "override_count_thresholds row ids are empty".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_cap_supports_only_confirmed_s_slots() {
        assert_eq!(
            mutation_action(&RankMutation::SetCap {
                rank: RankValue::SPlus,
                enabled: true,
            }),
            RankMutationAction::EasyCap { enabled: true }
        );
        assert!(matches!(
            mutation_action(&RankMutation::SetCap {
                rank: RankValue::A,
                enabled: true,
            }),
            RankMutationAction::Unsupported(_)
        ));
    }

    #[test]
    fn threshold_mutations_require_rows() {
        assert!(matches!(
            mutation_action(&RankMutation::ShiftCountThresholds {
                rank_row_ids: Vec::new(),
                source_prefix: [60_000, 60_000, 48_000],
                inserted_first: 72_000,
                inserted_second: None,
            }),
            RankMutationAction::Unsupported(_)
        ));
    }
}
