#![allow(dead_code)]

use sdk_schema::{schema, RegistryEventDescriptor, RegistryModuleSchema, SchemaTypeRef};

use crate::support::add_schema_types;

#[schema(constructible = false)]
struct RankSlot {
    slot_index: u64,
    rank_row_id: u64,
    raw_words: Vec<u64>,
    fixed_row_words: Option<Vec<u64>>,
    condition_row_id: Option<u64>,
    condition_row_words: Option<Vec<u64>>,
}

#[schema(constructible = false)]
struct RankSnapshot {
    global: u64,
    fixed_rank_table: u64,
    active_player: u64,
    mission_id: u64,
    mode_type: u64,
    difficulty: u64,
    slots: Vec<RankSlot>,
}

#[schema(constructible = false)]
struct RankResultEvent {
    schema: String,
    rank: String,
    count: Option<String>,
    time: Option<String>,
    merge: Option<String>,
    mission_id: Option<u64>,
    mode: Option<String>,
    difficulty: Option<String>,
}

#[schema(constructible = false)]
struct RankMergeScore {
    left_score_index: u64,
    right_score_index: u64,
    left_score: u64,
    right_score: u64,
    combined_score: u64,
    grade_score_indexes: Vec<u64>,
    grade_targets: Vec<u64>,
}

#[schema(constructible = false)]
struct RankHelperCall {
    kind: String,
    caller: String,
    caller_rva: Option<u64>,
    caller_label: String,
    row: Option<u64>,
    row_offset: Option<u64>,
    slot: Option<u64>,
    selectors: Option<Vec<u64>>,
    thresholds: Option<Vec<u64>>,
    all_thresholds: Option<Vec<Vec<u64>>>,
    value_f32: Option<f64>,
    value_u32: Option<u64>,
    divisor: Option<f64>,
    normalized: Option<u64>,
    left_rank: Option<u64>,
    left_label: Option<String>,
    right_rank: Option<u64>,
    right_label: Option<String>,
    result: u64,
    result_label: String,
    score: Option<RankMergeScore>,
}

pub fn rank_schema() -> RegistryModuleSchema {
    let mut schema = RegistryModuleSchema::new("sdk", "rank");
    add_schema_types::<RankSnapshot>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "snapshot",
        "sdk.runtime.rank.snapshot",
        RankSnapshot::schema_type_ref(),
    ));
    add_schema_types::<RankResultEvent>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "result",
        "sdk.runtime.rank.event",
        RankResultEvent::schema_type_ref(),
    ));
    add_schema_types::<RankHelperCall>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "calc_count",
        "sdk.runtime.rank.calc_count",
        RankHelperCall::schema_type_ref(),
    ));
    schema = schema.event(RegistryEventDescriptor::new(
        "calc_time",
        "sdk.runtime.rank.calc_time",
        RankHelperCall::schema_type_ref(),
    ));
    schema.event(RegistryEventDescriptor::new(
        "helper_call",
        "sdk.runtime.rank.helper_call",
        RankHelperCall::schema_type_ref(),
    ))
}
