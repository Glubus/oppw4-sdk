#![allow(dead_code)]

use sdk_schema::{schema, RegistryFunctionDescriptor, RegistryModuleSchema, RegistryTypeRef};

use crate::support::event_module_schema;

#[schema(constructible = false)]
struct MissionRewardMedal {
    item_id: u64,
    amount: u64,
    is_new: bool,
}

#[schema(constructible = false)]
struct MissionRewardsEvent {
    schema: String,
    rank: String,
    count: Option<String>,
    time: Option<String>,
    merge: Option<String>,
    berry: Option<u64>,
    crew_points: Option<u64>,
    medals: Vec<MissionRewardMedal>,
}

pub fn mission_schema() -> RegistryModuleSchema {
    event_module_schema::<MissionRewardsEvent>(
        RegistryModuleSchema::new("sdk", "mission").function(
            RegistryFunctionDescriptor::new("set_reward_berry_total", RegistryTypeRef::Void)
                .param("total", RegistryTypeRef::I64),
        ),
        "rewards",
        "sdk.runtime.rewards.event",
    )
}
