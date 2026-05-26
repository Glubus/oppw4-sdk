use mlua::Lua;

use crate::runtime::core::{
    events::RuntimeMutation,
    live_bus,
    rank::RankValue,
    rewards::{RewardCommitEvent, RewardMutation, RewardState},
};

use super::lua;

fn install_with_registry(lua_state: &Lua) -> lua::RewardLuaRegistry {
    lua_api::install_runtime(lua_state).expect("runtime");
    let registry = lua::RewardLuaRegistry::new();
    let module = lua::module_with_registry(lua_state, registry.clone()).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
    registry
}

#[test]
fn sdk_runtime_rewards_exposes_only_event_api() {
    let lua_state = Lua::new();
    install_with_registry(&lua_state);

    let values: (String, String) = lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            return type(rewards.on_commit), tostring(rewards.berry)
            "#,
        )
        .eval()
        .expect("api shape");

    assert_eq!(values, ("function".to_string(), "nil".to_string()));
}

#[test]
fn sdk_runtime_rewards_on_commit_accepts_function() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);

    lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            rewards.on_commit(function(ctx) end)
            "#,
        )
        .exec()
        .expect("register on_commit");

    assert_eq!(registry.len(), 1);
}

#[test]
fn sdk_runtime_rewards_on_commit_rank_contains_matches_runtime_rank() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            matched = false
            rewards.on_commit(function(ctx)
              matched = ctx.rank:contains({ "s", "s_plus" })
            end)
            "#,
        )
        .exec()
        .expect("register on_commit");

    let report = registry.dispatch_reward_commit(
        &lua_state,
        &RewardCommitEvent::new(RankValue::S, RewardState::new().with_berry(100)),
    );

    let matched: bool = lua_state.globals().get("matched").expect("matched");
    assert!(matched);
    assert_eq!(report.errors, []);
}

#[test]
fn sdk_runtime_rewards_on_commit_berry_multiply_produces_core_mutation() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            rewards.on_commit(function(ctx)
              if ctx.rank:contains({ "s", "s_plus" }) then
                ctx.rewards.berry:multiply(2)
              end
            end)
            "#,
        )
        .exec()
        .expect("register on_commit");

    let report = registry.dispatch_reward_commit(
        &lua_state,
        &RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100)),
    );

    assert_eq!(report.mutations, [RewardMutation::MultiplyBerry(2.0)]);
    assert_eq!(report.errors, []);
}

#[test]
fn sdk_runtime_rewards_on_commit_error_is_reported() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local rewards = require("sdk.runtime.rewards")
            rewards.on_commit(function(ctx)
              error("boom")
            end)
            "#,
        )
        .exec()
        .expect("register on_commit");

    let report = registry.dispatch_reward_commit(
        &lua_state,
        &RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100)),
    );

    assert_eq!(report.mutations, []);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].message.contains("boom"));
}

#[test]
fn sdk_runtime_rewards_on_commit_registers_live_bus_handler() {
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
                local rewards = require("sdk.runtime.rewards")
                rewards.on_commit(function(ctx)
                  ctx.rewards.berry:multiply(3)
                end)
                "#,
            )
            .exec()
            .expect("register live on_commit");
    }

    let report = live_bus::dispatch_runtime_event(
        RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100)).into(),
    );

    assert_eq!(
        report.mutations,
        [RuntimeMutation::Reward(RewardMutation::MultiplyBerry(3.0))]
    );
    assert_eq!(report.errors, []);
    live_bus::reset_runtime_handlers_for_tests();
}
