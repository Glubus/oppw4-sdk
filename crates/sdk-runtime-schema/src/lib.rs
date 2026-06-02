mod difficulty;
mod mission;
mod player;
mod rank;
mod rewards;
mod snapshot;
mod support;

pub use difficulty::difficulty_schema;
pub use mission::mission_schema;
pub use player::player_schema;
pub use rank::rank_schema;
pub use rewards::rewards_schema;
pub use snapshot::snapshot_schema;

use sdk_schema::RegistryModuleSchema;

pub fn runtime_schemas() -> Vec<RegistryModuleSchema> {
    vec![
        player_schema(),
        snapshot_schema(),
        difficulty_schema(),
        rank_schema(),
        rewards_schema(),
        mission_schema(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_schemas_validate_contracts() {
        for schema in runtime_schemas() {
            schema.validate_contract().unwrap_or_else(|error| {
                panic!("{} schema contract is invalid: {error}", schema.import_name)
            });
        }
    }

    #[test]
    fn player_schema_exposes_expected_function_and_event() {
        let schema = player_schema();
        assert_eq!(schema.import_name, "player");
        assert_eq!(schema.functions.len(), 1);
        assert_eq!(schema.functions[0].name, "active_characters");
        assert_eq!(schema.events.len(), 1);
        assert_eq!(schema.events[0].name, "character_changed");
    }

    #[test]
    fn difficulty_schema_exposes_expected_events() {
        let schema = difficulty_schema();
        assert_eq!(schema.import_name, "difficulty");
        assert_eq!(schema.events.len(), 2);
        assert_eq!(schema.events[0].name, "snapshot");
        assert_eq!(schema.events[1].name, "applied");
    }

    #[test]
    fn snapshot_schema_exposes_expected_functions() {
        let schema = snapshot_schema();
        assert_eq!(schema.import_name, "snapshot");
        assert_eq!(schema.functions.len(), 3);
        assert_eq!(schema.functions[0].name, "mission");
        assert_eq!(schema.functions[1].name, "difficulty");
        assert_eq!(schema.functions[2].name, "player");
    }

    #[test]
    fn rank_schema_exposes_expected_events() {
        let schema = rank_schema();
        assert_eq!(schema.import_name, "rank");
        assert_eq!(schema.events.len(), 5);
        assert_eq!(schema.events[0].name, "snapshot");
        assert_eq!(schema.events[1].name, "result");
        assert_eq!(schema.events[2].name, "calc_count");
        assert_eq!(schema.events[3].name, "calc_time");
        assert_eq!(schema.events[4].name, "helper_call");
    }

    #[test]
    fn rewards_schema_exposes_expected_events() {
        let schema = rewards_schema();
        assert_eq!(schema.import_name, "rewards");
        assert_eq!(schema.events.len(), 3);
        assert_eq!(schema.events[0].name, "commit");
        assert_eq!(schema.events[1].name, "event");
        assert_eq!(schema.events[2].name, "medals");
    }

    #[test]
    fn mission_schema_exposes_expected_function_and_event() {
        let schema = mission_schema();
        assert_eq!(schema.import_name, "mission");
        assert_eq!(schema.functions.len(), 1);
        assert_eq!(schema.functions[0].name, "set_reward_berry_total");
        assert_eq!(schema.events.len(), 1);
        assert_eq!(schema.events[0].name, "rewards");
    }
}
