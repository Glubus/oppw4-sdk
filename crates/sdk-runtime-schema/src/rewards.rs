#![allow(dead_code)]

use sdk_schema::{schema, RegistryEventDescriptor, RegistryModuleSchema, SchemaTypeRef};

use crate::support::add_schema_types;

#[schema(constructible = false)]
struct RewardCommitSnapshot {
    call: u64,
    reward_out: u64,
    reward_param: u64,
    mission_or_reward: u64,
    rank_or_mode: u64,
    bonus_a: u64,
    bonus_b: u64,
    slots: Vec<u64>,
}

#[schema(constructible = false)]
struct RewardEventItem {
    item_id: u64,
    amount: u64,
    is_new: bool,
}

#[schema(constructible = false)]
struct RewardEvent {
    schema: String,
    rank: String,
    count: Option<String>,
    time: Option<String>,
    merge: Option<String>,
    berry: Option<u64>,
    crew_points: Option<u64>,
    medals: Vec<RewardEventItem>,
}

#[schema(constructible = false)]
struct MedalRewardEntry {
    index: u64,
    amount: u64,
    item_id: u64,
    is_new: u64,
}

#[schema(constructible = false)]
struct MedalRewardSnapshot {
    call: u64,
    out: u64,
    reward_context: u64,
    previous: u64,
    result: u64,
    entries: Vec<MedalRewardEntry>,
}

pub fn rewards_schema() -> RegistryModuleSchema {
    let mut schema = RegistryModuleSchema::new("sdk", "rewards");
    add_schema_types::<RewardCommitSnapshot>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "commit",
        "sdk.runtime.rewards.commit",
        RewardCommitSnapshot::schema_type_ref(),
    ));
    add_schema_types::<RewardEvent>(&mut schema);
    schema = schema.event(RegistryEventDescriptor::new(
        "event",
        "sdk.runtime.rewards.event",
        RewardEvent::schema_type_ref(),
    ));
    add_schema_types::<MedalRewardSnapshot>(&mut schema);
    schema.event(RegistryEventDescriptor::new(
        "medals",
        "sdk.runtime.rewards.medals",
        MedalRewardSnapshot::schema_type_ref(),
    ))
}
