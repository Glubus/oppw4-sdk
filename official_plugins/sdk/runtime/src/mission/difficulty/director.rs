use plugin_sdk::OwnedHostApi;

use crate::runtime::{
    core::{
        difficulty::{DifficultyMutation, DifficultyValueOp},
        events::RuntimeMutation,
    },
    probe::PLUGIN_ID,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DifficultyApplyReport {
    pub(crate) accepted: usize,
    pub(crate) skipped: usize,
}

pub(crate) fn apply_difficulty_mutations(
    host: &OwnedHostApi,
    mutations: &[RuntimeMutation],
) -> DifficultyApplyReport {
    let mut report = DifficultyApplyReport::default();
    for mutation in mutations {
        let RuntimeMutation::Difficulty(mutation) = mutation else {
            continue;
        };
        match mutation_action(mutation) {
            DifficultyMutationAction::Accepted { target, operation } => {
                report.accepted += 1;
                let _ = host.log().write(
                    PLUGIN_ID,
                    format!(
                        "difficulty_runtime mutation accepted target={target} operation={operation} writer=pending"
                    ),
                );
            }
            DifficultyMutationAction::Skipped(reason) => {
                report.skipped += 1;
                let _ = host.log().write(
                    PLUGIN_ID,
                    format!("difficulty_runtime mutation skipped: {reason}"),
                );
            }
        }
    }
    report
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DifficultyMutationAction {
    Accepted { target: String, operation: String },
    Skipped(String),
}

fn mutation_action(mutation: &DifficultyMutation) -> DifficultyMutationAction {
    match mutation {
        DifficultyMutation::CombatPressure { operation } => DifficultyMutationAction::Accepted {
            target: "combat_pressure".to_string(),
            operation: format_operation(*operation),
        },
        DifficultyMutation::KnownTable { table, operation } if is_confirmed_known_table(table) => {
            DifficultyMutationAction::Accepted {
                target: table.clone(),
                operation: format_operation(*operation),
            }
        }
        DifficultyMutation::KnownTable { table, .. } => DifficultyMutationAction::Skipped(format!(
            "known_table target={table} is not confirmed for writes"
        )),
        DifficultyMutation::UnsupportedActorStat { stat } => {
            DifficultyMutationAction::Skipped(format!(
                "actor stat {stat} is not supported until hp/attack/defense reverse is confirmed"
            ))
        }
    }
}

fn is_confirmed_known_table(table: &str) -> bool {
    matches!(
        table,
        "combat_pressure"
            | "behavior_chance_a"
            | "behavior_chance_b"
            | "spawn_b3_a"
            | "spawn_b3_b"
            | "spawn_b3_c"
            | "candidate_a"
            | "candidate_b"
    )
}

fn format_operation(operation: DifficultyValueOp) -> String {
    match operation {
        DifficultyValueOp::SetF32(value) => format!("set_f32:{value}"),
        DifficultyValueOp::AddF32(value) => format!("add_f32:{value}"),
        DifficultyValueOp::ScaleF32(value) => format!("scale_f32:{value}"),
        DifficultyValueOp::SetU16(value) => format!("set_u16:{value}"),
        DifficultyValueOp::AddI16(value) => format!("add_i16:{value}"),
        DifficultyValueOp::SetU8(value) => format!("set_u8:{value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_combat_pressure_and_confirmed_tables() {
        assert_eq!(
            mutation_action(&DifficultyMutation::CombatPressure {
                operation: DifficultyValueOp::ScaleF32(1.5),
            }),
            DifficultyMutationAction::Accepted {
                target: "combat_pressure".to_string(),
                operation: "scale_f32:1.5".to_string(),
            }
        );
        assert!(matches!(
            mutation_action(&DifficultyMutation::KnownTable {
                table: "spawn_b3_a".to_string(),
                operation: DifficultyValueOp::SetU8(60),
            }),
            DifficultyMutationAction::Accepted { .. }
        ));
    }

    #[test]
    fn skips_actor_stats_and_unknown_tables() {
        assert!(matches!(
            mutation_action(&DifficultyMutation::UnsupportedActorStat {
                stat: "attack".to_string(),
            }),
            DifficultyMutationAction::Skipped(_)
        ));
        assert!(matches!(
            mutation_action(&DifficultyMutation::KnownTable {
                table: "hp".to_string(),
                operation: DifficultyValueOp::ScaleF32(2.0),
            }),
            DifficultyMutationAction::Skipped(_)
        ));
    }
}
