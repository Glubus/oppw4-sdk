use mlua::Lua;
use plugin_sdk::{RankCapEffect, RankCapRule, RankCondition, RankConditionExpr, RankSlot};

use super::lua;

fn install(lua_state: &Lua) {
    lua_api::install_runtime(lua_state).expect("runtime");
    let module = lua::module(lua_state).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
    let player_module =
        crate::runtime::player::lua_module_for_test(lua_state).expect("player module");
    lua_api::register_module(
        lua_state,
        crate::runtime::player::LUA_MODULE_NAME_FOR_TEST,
        player_module,
    )
    .expect("register player");
}

#[test]
fn sdk_runtime_ranks_builds_unconditional_enable_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local ranks = require("sdk.runtime.ranks")
            return ranks.slot({4, "s+"}):condition(nil):enable()
            "#,
        )
        .eval()
        .expect("rule json");
    let rule: RankCapRule = serde_json::from_str(&json).expect("rule");

    assert_eq!(rule.slots, [RankSlot::s(), RankSlot::s_plus()]);
    assert_eq!(rule.condition, RankConditionExpr::None);
    assert_eq!(rule.effect, RankCapEffect::Enable);
}

#[test]
fn sdk_runtime_ranks_builds_all_condition_disable_rule() {
    let lua_state = Lua::new();
    install(&lua_state);

    let json: String = lua_state
        .load(
            r#"
            local ranks = require("sdk.runtime.ranks")
            local player = require("sdk.runtime.player")
            return ranks.slot("d")
              :condition(ranks.all(
                player:active_character("zoro"),
                ranks.flag("crew.elbaph", true)
              ))
              :disable()
            "#,
        )
        .eval()
        .expect("rule json");
    let rule: RankCapRule = serde_json::from_str(&json).expect("rule");

    assert_eq!(rule.slots, [RankSlot::d()]);
    assert_eq!(
        rule.condition,
        RankConditionExpr::all([
            RankCondition::active_character("zoro"),
            RankCondition::flag("crew.elbaph", true),
        ])
    );
    assert_eq!(rule.effect, RankCapEffect::Disable);
}
