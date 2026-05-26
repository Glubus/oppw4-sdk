use mlua::Lua;

use crate::runtime::core::{
    events::RuntimeMutation,
    live_bus,
    rank::{RankMutation, RankResultEvent, RankValue},
};

use super::lua;

fn install_with_registry(lua_state: &Lua) -> lua::RankLuaRegistry {
    lua_api::install_runtime(lua_state).expect("runtime");
    let registry = lua::RankLuaRegistry::new();
    let module = lua::module_with_registry(lua_state, registry.clone()).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
    registry
}

#[test]
fn sdk_runtime_ranks_exposes_only_event_api() {
    let lua_state = Lua::new();
    install_with_registry(&lua_state);

    let values: (String, String) = lua_state
        .load(
            r#"
            local ranks = require("sdk.runtime.ranks")
            return type(ranks.on_result), tostring(ranks.slot)
            "#,
        )
        .eval()
        .expect("api shape");

    assert_eq!(values, ("function".to_string(), "nil".to_string()));
}

#[test]
fn sdk_runtime_ranks_on_result_produces_core_cap_mutation() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local ranks = require("sdk.runtime.ranks")
            ranks.on_result(function(ctx)
              if ctx.rank:contains({ "S+" }) then
                ctx.rank:set_cap("s_plus", true)
              end
            end)
            "#,
        )
        .exec()
        .expect("register on_result");

    let report = registry.dispatch_rank_result(&lua_state, &RankResultEvent::new(RankValue::SPlus));

    assert_eq!(registry.len(), 1);
    assert_eq!(
        report.mutations,
        [RankMutation::SetCap {
            rank: RankValue::SPlus,
            enabled: true,
        }]
    );
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn sdk_runtime_ranks_on_result_registers_live_bus_handler() {
    let _guard = live_bus::test_lock();
    live_bus::reset_runtime_handlers_for_tests();
    {
        let lua_state = Lua::new();
        lua_state
            .globals()
            .set("__oppw4_runtime_live_callbacks", true)
            .expect("enable live callbacks");
        install_with_registry(&lua_state);
        lua_state
            .load(
                r#"
                local ranks = require("sdk.runtime.ranks")
                ranks.on_result(function(ctx)
                  ctx.rank:set_cap("s_plus", true)
                end)
                "#,
            )
            .exec()
            .expect("register live on_result");
    }

    let report = live_bus::dispatch_runtime_event(RankResultEvent::new(RankValue::SPlus).into());

    assert_eq!(
        report.mutations,
        [RuntimeMutation::Rank(RankMutation::SetCap {
            rank: RankValue::SPlus,
            enabled: true,
        })]
    );
    assert_eq!(report.errors, []);
    live_bus::reset_runtime_handlers_for_tests();
}
