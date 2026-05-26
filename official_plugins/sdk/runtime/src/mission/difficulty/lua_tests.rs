use mlua::Lua;
use plugin_sdk::{
    DifficultyAction, DifficultyCondition, DifficultyConditionExpr, DifficultyRule,
    DifficultyValueOp,
};

use super::lua;

fn install(lua_state: &Lua) {
    lua_api::install_runtime(lua_state).expect("runtime");
    let module = lua::module(lua_state).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
}

#[test]
fn sdk_runtime_difficulty_builds_unconditional_enable_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return difficulty.level({0, "super-hard"}):condition(nil):enable()
            "#,
        )
        .eval()
        .expect("rule json");
    let rule: DifficultyRule = serde_json::from_str(&json).expect("rule");

    assert_eq!(
        rule.levels
            .iter()
            .map(|level| level.as_str())
            .collect::<Vec<_>>(),
        ["easy", "super_hard"]
    );
    assert_eq!(rule.condition, DifficultyConditionExpr::None);
    assert_eq!(rule.action, DifficultyAction::EnableLevel);
}

#[test]
fn sdk_runtime_difficulty_builds_all_condition_disable_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return difficulty.level("hard")
              :condition(difficulty.all(
                difficulty.active_character("zoro"),
                difficulty.flag("crew.elbaph", true)
              ))
              :disable()
            "#,
        )
        .eval()
        .expect("rule json");
    let rule: DifficultyRule = serde_json::from_str(&json).expect("rule");

    assert_eq!(rule.levels[0].as_str(), "hard");
    assert_eq!(
        rule.condition,
        DifficultyConditionExpr::all([
            DifficultyCondition::active_character("zoro"),
            DifficultyCondition::flag("crew.elbaph", true),
        ])
    );
    assert_eq!(rule.action, DifficultyAction::DisableLevel);
}

#[test]
fn sdk_runtime_difficulty_builds_actor_stat_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return difficulty.level("super-hard")
              :stat("defense")
              :multiply(1.35)
            "#,
        )
        .eval()
        .expect("rule json");
    let rule: DifficultyRule = serde_json::from_str(&json).expect("rule");

    assert_eq!(rule.levels[0].as_str(), "super_hard");
    assert_eq!(
        rule.action,
        DifficultyAction::ActorStat {
            stat: "defense".into(),
            operation: DifficultyValueOp::ScaleF32(1.35),
        }
    );
}

#[test]
fn sdk_runtime_difficulty_builds_open_table_rules() {
    let lua_state = Lua::new();
    install(&lua_state);

    let table_json: String = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return difficulty.level("hard")
              :table("0xb3d8")
              :set_u8(60)
            "#,
        )
        .eval()
        .expect("table rule json");
    let table_rule: DifficultyRule = serde_json::from_str(&table_json).expect("table rule");

    assert_eq!(
        table_rule.action,
        DifficultyAction::KnownTable {
            table: "spawn_b3_a".into(),
            operation: DifficultyValueOp::SetU8(60),
        }
    );

    let raw_json: String = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return difficulty.level("hard")
              :raw("fixed20", "0xc57c")
              :set(9)
            "#,
        )
        .eval()
        .expect("raw rule json");
    let raw_rule: DifficultyRule = serde_json::from_str(&raw_json).expect("raw rule");

    assert_eq!(
        raw_rule.action,
        DifficultyAction::RawFixedTable {
            area: "fixed20".into(),
            offset: 0xc57c,
            operation: DifficultyValueOp::SetF32(9.0),
        }
    );
}
