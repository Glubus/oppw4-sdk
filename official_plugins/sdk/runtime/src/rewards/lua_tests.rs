use mlua::Lua;
use serde_json::json;

use super::lua;

fn install(lua_state: &Lua) {
    lua_api::install_runtime(lua_state).expect("runtime");
    let module = lua::module(lua_state).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
}

#[test]
fn sdk_runtime_rewards_is_requireable() {
    let lua_state = Lua::new();
    install(&lua_state);

    let is_table: bool = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return type(rewards.missions()) == "table"
            "#,
        )
        .eval()
        .expect("rewards module");

    assert!(is_table);
}

#[test]
fn sdk_runtime_rewards_returns_nil_for_unknown_mission() {
    let lua_state = Lua::new();
    install(&lua_state);

    let missing: bool = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.for_mission("missing_mission") == nil
            "#,
        )
        .eval()
        .expect("missing mission");

    assert!(missing);
}

#[test]
fn sdk_runtime_rewards_builds_force_add_souls_stub_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.souls
              :condition(rewards.flag("mission.elbaph", true))
              :force_add({ l_atk = 0, l_def = 1, l_hp = 4 })
            "#,
        )
        .eval()
        .expect("force add rule");
    let rule: serde_json::Value = serde_json::from_str(&json).expect("rule json");

    assert_eq!(
        rule,
        json!({
            "kind": "reward_rule",
            "target": "souls",
            "action": {
                "type": "force_add",
                "missing_only": true,
                "minimum": 1,
                "rewards": {
                    "reward_souls": {
                        "souls": [
                            {
                                "type": "l_atk",
                                "count": 1
                            },
                            {
                                "type": "l_def",
                                "count": 1
                            },
                            {
                                "type": "l_hp",
                                "count": 4
                            }
                        ]
                    }
                }
            },
            "condition": {
                "mode": "all",
                "conditions": [
                    {
                        "kind": "flag",
                        "key": "mission.elbaph",
                        "value": true
                    }
                ]
            },
            "enabled": true,
            "stub": true
        })
    );
}

#[test]
fn sdk_runtime_rewards_builds_multiply_souls_stub_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.souls
              :condition(rewards.any(
                rewards.equals("mission.mode", "treasure_log"),
                rewards.custom("event", "bonus")
              ))
              :multiply(3)
            "#,
        )
        .eval()
        .expect("multiply rule");
    let rule: serde_json::Value = serde_json::from_str(&json).expect("rule json");

    assert_eq!(rule["action"], json!({ "type": "multiply", "factor": 3.0 }));
    assert_eq!(rule["condition"]["mode"], "any");
    assert_eq!(rule["condition"]["conditions"][0]["kind"], "equals");
    assert_eq!(rule["stub"], true);
}

#[test]
fn sdk_runtime_rewards_builds_rank_contains_condition() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            local rank = rewards.rank
            return rewards.berry
              :condition(rank:contains({ "S", "S+" }))
              :multiply(2)
            "#,
        )
        .eval()
        .expect("rank condition rule");
    let rule: serde_json::Value = serde_json::from_str(&json).expect("rule json");

    assert_eq!(rule["target"], "berry");
    assert_eq!(rule["action"], json!({ "type": "multiply", "factor": 2.0 }));
    assert_eq!(
        rule["condition"],
        json!({
            "mode": "all",
            "conditions": [
                {
                    "kind": "rank_contains",
                    "slots": ["s", "s_plus"]
                }
            ]
        })
    );
}

#[test]
fn sdk_runtime_rewards_builds_typed_reward_objects() {
    let lua_state = Lua::new();
    install(&lua_state);

    let berry_json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.berry:force_add(0)
            "#,
        )
        .eval()
        .expect("berry rule");
    let crew_json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.crew_points:force_add({ amount = 12 })
            "#,
        )
        .eval()
        .expect("crew point rule");
    let medals_json: String = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return rewards.medals:force_add({ gold = 2, silver = 0 })
            "#,
        )
        .eval()
        .expect("medals rule");

    let berry: serde_json::Value = serde_json::from_str(&berry_json).expect("berry json");
    let crew: serde_json::Value = serde_json::from_str(&crew_json).expect("crew json");
    let medals: serde_json::Value = serde_json::from_str(&medals_json).expect("medals json");

    assert_eq!(
        berry["action"]["rewards"]["reward_berry"],
        json!({ "amount": 1 })
    );
    assert_eq!(
        crew["action"]["rewards"]["reward_crew_points"],
        json!({ "amount": 12 })
    );
    assert_eq!(
        medals["action"]["rewards"]["reward_medals"],
        json!({
            "medals": [
                { "type": "gold", "count": 2 },
                { "type": "silver", "count": 1 }
            ]
        })
    );
}
