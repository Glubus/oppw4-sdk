#![allow(dead_code)]

use sdk_schema::{schema, RegistryFunctionDescriptor, RegistryModuleSchema, RegistryTypeRef};

use crate::support::event_module_schema;

#[schema(constructible = false)]
struct CharacterChangedEvent {
    previous_character_id: Option<String>,
    current_character_id: String,
    active_character_ids: Vec<String>,
}

pub fn player_schema() -> RegistryModuleSchema {
    event_module_schema::<CharacterChangedEvent>(
        RegistryModuleSchema::new("sdk", "player").function(RegistryFunctionDescriptor::new(
            "active_characters",
            RegistryTypeRef::Array {
                inner: Box::new(RegistryTypeRef::String),
            },
        )),
        "character_changed",
        "sdk.runtime.player.character_changed",
    )
}
