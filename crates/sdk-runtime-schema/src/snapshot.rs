#![allow(dead_code)]

use sdk_schema::{schema, RegistryFunctionDescriptor, RegistryModuleSchema, SchemaTypeRef};

use crate::support::add_schema_types;

#[schema(constructible = false)]
struct SnapshotMission {
    id: Option<u64>,
    mode: Option<String>,
}

#[schema(constructible = false)]
struct SnapshotDifficulty {
    key: Option<String>,
}

#[schema(constructible = false)]
struct SnapshotPlayer {
    active_character_ids: Vec<String>,
}

pub fn snapshot_schema() -> RegistryModuleSchema {
    let mut schema = RegistryModuleSchema::new("sdk", "snapshot");
    add_schema_types::<SnapshotMission>(&mut schema);
    add_schema_types::<SnapshotDifficulty>(&mut schema);
    add_schema_types::<SnapshotPlayer>(&mut schema);
    schema
        .function(RegistryFunctionDescriptor::new(
            "mission",
            SnapshotMission::schema_type_ref(),
        ))
        .function(RegistryFunctionDescriptor::new(
            "difficulty",
            SnapshotDifficulty::schema_type_ref(),
        ))
        .function(RegistryFunctionDescriptor::new(
            "player",
            SnapshotPlayer::schema_type_ref(),
        ))
}
