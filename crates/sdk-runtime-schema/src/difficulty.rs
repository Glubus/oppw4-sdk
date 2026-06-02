#![allow(dead_code)]

use sdk_schema::{schema, RegistryEventDescriptor, RegistryModuleSchema, SchemaTypeRef};

use crate::support::add_schema_types;

#[schema(constructible = false)]
struct DifficultySnapshot {
    module_base: u64,
    global: u64,
    mission_id: u64,
    mode_type: u64,
    reward_mode: u64,
    difficulty: u64,
    special_flag: u64,
    cached_mission: u64,
    cached_difficulty: u64,
}

#[schema(constructible = false)]
struct DifficultyAppliedEvent {
    schema: String,
    mission_id: Option<u64>,
    mode: String,
    difficulty: String,
}

pub fn difficulty_schema() -> RegistryModuleSchema {
    let mut schema = RegistryModuleSchema::new("sdk", "difficulty");
    add_schema_types::<DifficultySnapshot>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "snapshot",
        "sdk.runtime.difficulty.snapshot",
        DifficultySnapshot::schema_type_ref(),
    ));
    add_schema_types::<DifficultyAppliedEvent>(&mut schema);
    schema.event(RegistryEventDescriptor::new(
        "applied",
        "sdk.runtime.difficulty.event",
        DifficultyAppliedEvent::schema_type_ref(),
    ))
}
