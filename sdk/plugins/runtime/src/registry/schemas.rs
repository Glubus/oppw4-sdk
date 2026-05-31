pub(super) const PLAYER_SCHEMA_JSON: &str = r#"{
  "namespace": "sdk",
  "import_name": "player",
  "constructible": false,
  "functions": [
    {
      "name": "active_characters",
      "params": [],
      "returns": {
        "kind": "array",
        "inner": { "kind": "string" }
      }
    }
  ],
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

pub(super) const DIFFICULTY_SCHEMA_JSON: &str = r#"{
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

pub(super) const RANK_SCHEMA_JSON: &str = r#"{
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

pub(super) const REWARDS_SCHEMA_JSON: &str = r#"{
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
