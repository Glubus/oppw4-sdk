use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

const BERRY_TOTAL_SLOT: usize = 6;

static RULES: OnceLock<RwLock<Vec<RewardRule>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize)]
pub(super) struct RewardRule {
    target: String,
    action: RewardAction,
    condition: RewardConditionExpr,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RewardAction {
    ForceAdd {
        #[allow(dead_code)]
        missing_only: bool,
        #[allow(dead_code)]
        minimum: u32,
        #[allow(dead_code)]
        rewards: serde_json::Value,
    },
    Multiply {
        factor: f64,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "mode", content = "conditions", rename_all = "snake_case")]
enum RewardConditionExpr {
    None,
    All(Vec<RewardCondition>),
    Any(Vec<RewardCondition>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RewardCondition {
    RankContains {
        slots: Vec<String>,
    },
    Flag {
        #[allow(dead_code)]
        key: String,
        #[allow(dead_code)]
        value: bool,
    },
    Equals {
        #[allow(dead_code)]
        key: String,
        #[allow(dead_code)]
        value: String,
    },
    Custom {
        #[allow(dead_code)]
        key: String,
        #[allow(dead_code)]
        value: String,
    },
}

pub(super) fn stage(rule: RewardRule) {
    rules().write().expect("reward rules lock").push(rule);
}

pub(super) fn apply_reward_commit(reward_out: *mut u64, rank_or_mode: i32) {
    if reward_out.is_null() {
        return;
    }

    let multiplier = rules()
        .read()
        .expect("reward rules lock")
        .iter()
        .filter(|rule| rule.enabled && rule.target == "berry")
        .filter_map(|rule| match rule.action {
            RewardAction::Multiply { factor }
                if matches_condition(&rule.condition, rank_or_mode) =>
            {
                Some(factor)
            }
            _ => None,
        })
        .fold(1.0_f64, |total, factor| total * factor.max(1.0));

    if multiplier <= 1.0 {
        return;
    }

    unsafe {
        let slot = reward_out.add(BERRY_TOTAL_SLOT);
        let current = slot.read();
        slot.write(scale_u64(current, multiplier));
    }
}

fn rules() -> &'static RwLock<Vec<RewardRule>> {
    RULES.get_or_init(|| RwLock::new(Vec::new()))
}

fn matches_condition(condition: &RewardConditionExpr, rank_or_mode: i32) -> bool {
    match condition {
        RewardConditionExpr::None => true,
        RewardConditionExpr::All(conditions) => conditions
            .iter()
            .all(|condition| matches_single_condition(condition, rank_or_mode)),
        RewardConditionExpr::Any(conditions) => conditions
            .iter()
            .any(|condition| matches_single_condition(condition, rank_or_mode)),
    }
}

fn matches_single_condition(condition: &RewardCondition, rank_or_mode: i32) -> bool {
    match condition {
        RewardCondition::RankContains { slots } => {
            let Some(rank) = rank_slot(rank_or_mode) else {
                return false;
            };
            slots.iter().any(|slot| slot == rank)
        }
        RewardCondition::Flag { .. }
        | RewardCondition::Equals { .. }
        | RewardCondition::Custom { .. } => false,
    }
}

fn rank_slot(rank_or_mode: i32) -> Option<&'static str> {
    match rank_or_mode {
        0 => Some("d"),
        1 => Some("c"),
        2 => Some("b"),
        3 => Some("a"),
        4 => Some("s"),
        5 => Some("s_plus"),
        _ => None,
    }
}

fn scale_u64(value: u64, factor: f64) -> u64 {
    if !factor.is_finite() {
        return value;
    }
    ((value as f64) * factor)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64
}

fn enabled_by_default() -> bool {
    true
}
