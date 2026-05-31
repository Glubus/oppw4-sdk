mod config;
mod game;
mod mission;
mod reverse;
mod rewards;
mod runtime;

use std::{
    ffi::{c_char, c_void},
    ptr,
};

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        runtime::Runtime::initialize(context.host())?;
        register_runtime_modules(context)?;
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

struct RuntimeModule {
    name: &'static str,
    schema_json: &'static str,
}

const RUNTIME_MODULES: &[RuntimeModule] = &[
    RuntimeModule {
        name: "sdk.player",
        schema_json: PLAYER_SCHEMA_JSON,
    },
    RuntimeModule {
        name: "sdk.difficulty",
        schema_json: DIFFICULTY_SCHEMA_JSON,
    },
    RuntimeModule {
        name: "sdk.rank",
        schema_json: RANK_SCHEMA_JSON,
    },
    RuntimeModule {
        name: "sdk.rewards",
        schema_json: REWARDS_SCHEMA_JSON,
    },
];

const PLAYER_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "player",
  "constructible": false,
  "functions": [],
  "types": [
    {
      "name": "CharacterChangedEvent",
      "constructible": false,
      "fields": [
        { "name": "characterId", "type_ref": { "kind": "string" } },
        { "name": "activeCharacterIds", "type_ref": { "kind": "array", "inner": { "kind": "string" } } }
      ]
    }
  ],
  "events": [
    {
      "name": "character_changed",
      "key": "sdk.runtime.player.character_changed",
      "payload": { "kind": "named", "name": "CharacterChangedEvent" }
    }
  ]
}"#;

const DIFFICULTY_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "difficulty",
  "constructible": false,
  "functions": [],
  "types": [
    {
      "name": "DifficultySnapshot",
      "constructible": false,
      "fields": [
        { "name": "module_base", "type_ref": { "kind": "i64" } },
        { "name": "global", "type_ref": { "kind": "i64" } },
        { "name": "mission_id", "type_ref": { "kind": "i64" } },
        { "name": "mode_type", "type_ref": { "kind": "i64" } },
        { "name": "reward_mode", "type_ref": { "kind": "i64" } },
        { "name": "difficulty", "type_ref": { "kind": "i64" } },
        { "name": "special_flag", "type_ref": { "kind": "i64" } },
        { "name": "cached_mission", "type_ref": { "kind": "i64" } },
        { "name": "cached_difficulty", "type_ref": { "kind": "i64" } }
      ]
    },
    {
      "name": "DifficultyAppliedEvent",
      "constructible": false,
      "fields": [
        { "name": "schema", "type_ref": { "kind": "string" } },
        { "name": "mission_id", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "mode", "type_ref": { "kind": "string" } },
        { "name": "difficulty", "type_ref": { "kind": "string" } }
      ]
    }
  ],
  "events": [
    {
      "name": "snapshot",
      "key": "sdk.runtime.difficulty.snapshot",
      "payload": { "kind": "named", "name": "DifficultySnapshot" }
    },
    {
      "name": "applied",
      "key": "sdk.runtime.difficulty.event",
      "payload": { "kind": "named", "name": "DifficultyAppliedEvent" }
    }
  ]
}"#;

const RANK_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "rank",
  "constructible": false,
  "functions": [],
  "types": [
    {
      "name": "RankSnapshot",
      "constructible": false,
      "fields": [
        { "name": "global", "type_ref": { "kind": "i64" } },
        { "name": "fixed_rank_table", "type_ref": { "kind": "i64" } },
        { "name": "active_player", "type_ref": { "kind": "i64" } },
        { "name": "mission_id", "type_ref": { "kind": "i64" } },
        { "name": "mode_type", "type_ref": { "kind": "i64" } },
        { "name": "difficulty", "type_ref": { "kind": "i64" } },
        { "name": "slots", "type_ref": { "kind": "array", "inner": { "kind": "named", "name": "RankSlot" } } }
      ]
    },
    {
      "name": "RankSlot",
      "constructible": false,
      "fields": [
        { "name": "slot_index", "type_ref": { "kind": "i64" } },
        { "name": "rank_row_id", "type_ref": { "kind": "i64" } },
        { "name": "raw_words", "type_ref": { "kind": "array", "inner": { "kind": "i64" } } },
        { "name": "fixed_row_words", "type_ref": { "kind": "optional", "inner": { "kind": "array", "inner": { "kind": "i64" } } } },
        { "name": "condition_row_id", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "condition_row_words", "type_ref": { "kind": "optional", "inner": { "kind": "array", "inner": { "kind": "i64" } } } }
      ]
    },
    {
      "name": "RankResultEvent",
      "constructible": false,
      "fields": [
        { "name": "schema", "type_ref": { "kind": "string" } },
        { "name": "rank", "type_ref": { "kind": "string" } },
        { "name": "mission_id", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "mode", "type_ref": { "kind": "optional", "inner": { "kind": "string" } } },
        { "name": "difficulty", "type_ref": { "kind": "optional", "inner": { "kind": "string" } } }
      ]
    },
    {
      "name": "RankHelperCall",
      "constructible": false,
      "fields": [
        { "name": "kind", "type_ref": { "kind": "string" } },
        { "name": "caller", "type_ref": { "kind": "string" } },
        { "name": "caller_rva", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "caller_label", "type_ref": { "kind": "string" } },
        { "name": "row", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "row_offset", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "slot", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "selectors", "type_ref": { "kind": "optional", "inner": { "kind": "array", "inner": { "kind": "i64" } } } },
        { "name": "thresholds", "type_ref": { "kind": "optional", "inner": { "kind": "array", "inner": { "kind": "i64" } } } },
        { "name": "all_thresholds", "type_ref": { "kind": "optional", "inner": { "kind": "array", "inner": { "kind": "array", "inner": { "kind": "i64" } } } } },
        { "name": "value_f32", "type_ref": { "kind": "optional", "inner": { "kind": "f64" } } },
        { "name": "value_u32", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "divisor", "type_ref": { "kind": "optional", "inner": { "kind": "f64" } } },
        { "name": "normalized", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "left_rank", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "left_label", "type_ref": { "kind": "optional", "inner": { "kind": "string" } } },
        { "name": "right_rank", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } },
        { "name": "right_label", "type_ref": { "kind": "optional", "inner": { "kind": "string" } } },
        { "name": "result", "type_ref": { "kind": "i64" } },
        { "name": "result_label", "type_ref": { "kind": "string" } },
        { "name": "score", "type_ref": { "kind": "optional", "inner": { "kind": "named", "name": "RankMergeScore" } } }
      ]
    },
    {
      "name": "RankMergeScore",
      "constructible": false,
      "fields": [
        { "name": "left_score_index", "type_ref": { "kind": "i64" } },
        { "name": "right_score_index", "type_ref": { "kind": "i64" } },
        { "name": "left_score", "type_ref": { "kind": "i64" } },
        { "name": "right_score", "type_ref": { "kind": "i64" } },
        { "name": "combined_score", "type_ref": { "kind": "i64" } },
        { "name": "grade_score_indexes", "type_ref": { "kind": "array", "inner": { "kind": "i64" } } },
        { "name": "grade_targets", "type_ref": { "kind": "array", "inner": { "kind": "i64" } } }
      ]
    }
  ],
  "events": [
    {
      "name": "snapshot",
      "key": "sdk.runtime.rank.snapshot",
      "payload": { "kind": "named", "name": "RankSnapshot" }
    },
    {
      "name": "result",
      "key": "sdk.runtime.rank.event",
      "payload": { "kind": "named", "name": "RankResultEvent" }
    },
    {
      "name": "helper_call",
      "key": "sdk.runtime.rank.helper_call",
      "payload": { "kind": "named", "name": "RankHelperCall" }
    }
  ]
}"#;

const REWARDS_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "rewards",
  "constructible": false,
  "functions": [],
  "types": [
    {
      "name": "RewardCommitSnapshot",
      "constructible": false,
      "fields": [
        { "name": "call", "type_ref": { "kind": "i64" } },
        { "name": "reward_out", "type_ref": { "kind": "i64" } },
        { "name": "reward_param", "type_ref": { "kind": "i64" } },
        { "name": "mission_or_reward", "type_ref": { "kind": "i64" } },
        { "name": "rank_or_mode", "type_ref": { "kind": "i64" } },
        { "name": "bonus_a", "type_ref": { "kind": "i64" } },
        { "name": "bonus_b", "type_ref": { "kind": "i64" } },
        { "name": "slots", "type_ref": { "kind": "array", "inner": { "kind": "i64" } } }
      ]
    },
    {
      "name": "RewardEvent",
      "constructible": false,
      "fields": [
        { "name": "schema", "type_ref": { "kind": "string" } },
        { "name": "rank", "type_ref": { "kind": "string" } },
        { "name": "berry", "type_ref": { "kind": "optional", "inner": { "kind": "i64" } } }
      ]
    },
    {
      "name": "ItemRewardSnapshot",
      "constructible": false,
      "fields": [
        { "name": "call", "type_ref": { "kind": "i64" } },
        { "name": "out", "type_ref": { "kind": "i64" } },
        { "name": "reward_context", "type_ref": { "kind": "i64" } },
        { "name": "previous", "type_ref": { "kind": "i64" } },
        { "name": "result", "type_ref": { "kind": "i64" } },
        { "name": "entries", "type_ref": { "kind": "array", "inner": { "kind": "named", "name": "ItemRewardEntry" } } }
      ]
    },
    {
      "name": "ItemRewardEntry",
      "constructible": false,
      "fields": [
        { "name": "index", "type_ref": { "kind": "i64" } },
        { "name": "amount", "type_ref": { "kind": "i64" } },
        { "name": "item_id", "type_ref": { "kind": "i64" } },
        { "name": "is_new", "type_ref": { "kind": "i64" } }
      ]
    }
  ],
  "events": [
    {
      "name": "commit",
      "key": "sdk.runtime.rewards.commit",
      "payload": { "kind": "named", "name": "RewardCommitSnapshot" }
    },
    {
      "name": "event",
      "key": "sdk.runtime.rewards.event",
      "payload": { "kind": "named", "name": "RewardEvent" }
    },
    {
      "name": "items",
      "key": "sdk.runtime.rewards.items",
      "payload": { "kind": "named", "name": "ItemRewardSnapshot" }
    }
  ]
}"#;

fn register_runtime_modules(context: PluginContext<'_>) -> PluginResult<()> {
    for module in RUNTIME_MODULES {
        register_runtime_module(context, module)?;
    }
    Ok(())
}

fn register_runtime_module(context: PluginContext<'_>, module: &RuntimeModule) -> PluginResult<()> {
    context.register_registry_module_with_schema(
        module.name,
        ptr::null_mut(),
        noop_module_install,
        module.schema_json,
        module_invoke,
    )
}

unsafe extern "system" fn noop_module_install(
    _module_context: *mut c_void,
    _runtime_context: *mut c_void,
) -> i32 {
    0
}

unsafe extern "system" fn module_invoke(
    _module_context: *mut c_void,
    _function_name_utf8: *const c_char,
    _args_json: *const u8,
    _args_json_len: usize,
    _out_json: *mut u8,
    _out_json_len: *mut usize,
) -> i32 {
    -42
}

export_plugin!(SdkRuntime);

#[cfg(test)]
mod tests {
    use sdk_bridge::RegistryModuleSchema;

    use super::RUNTIME_MODULES;

    #[test]
    fn runtime_module_schemas_are_valid_registry_contracts() {
        for module in RUNTIME_MODULES {
            let schema = serde_json::from_str::<RegistryModuleSchema>(module.schema_json)
                .unwrap_or_else(|error| panic!("{} schema is invalid: {error}", module.name));
            schema.validate_contract().unwrap_or_else(|error| {
                panic!("{} schema contract is invalid: {error}", module.name)
            });
        }
    }
}
