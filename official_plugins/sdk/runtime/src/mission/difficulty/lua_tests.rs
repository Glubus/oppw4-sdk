use mlua::Lua;

use crate::runtime::core::{
    difficulty::{
        DifficultyApplyEvent, DifficultyId, DifficultyMode, DifficultyMutation, DifficultySnapshot,
        DifficultyValueOp as CoreDifficultyValueOp,
    },
    events::RuntimeMutation,
    live_bus,
};

use super::lua;

fn install_with_registry(lua_state: &Lua) -> lua::DifficultyLuaRegistry {
    lua_api::install_runtime(lua_state).expect("runtime");
    let registry = lua::DifficultyLuaRegistry::new();
    let module = lua::module_with_registry(lua_state, registry.clone()).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
    registry
}

#[test]
fn sdk_runtime_difficulty_exposes_only_event_api() {
    let lua_state = Lua::new();
    install_with_registry(&lua_state);

    let values: (String, String) = lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            return type(difficulty.on_apply), tostring(difficulty.level)
            "#,
        )
        .eval()
        .expect("api shape");

    assert_eq!(values, ("function".to_string(), "nil".to_string()));
}

#[test]
fn sdk_runtime_difficulty_on_apply_produces_core_combat_pressure_mutation() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local difficulty = require("sdk.runtime.difficulty")
            difficulty.on_apply(function(ctx)
              if ctx.difficulty == "super_hard" then
                ctx.combat_pressure:multiply(1.5)
              end
            end)
            "#,
        )
        .exec()
        .expect("register on_apply");

    let event = DifficultyApplyEvent::new(
        DifficultySnapshot::new(
            DifficultyMode::new("treasure_log"),
            DifficultyId::new("super_hard"),
        )
        .with_mission_id(35),
    );
    let report = registry.dispatch_difficulty_apply(&lua_state, &event);

    assert_eq!(registry.len(), 1);
    assert_eq!(
        report.mutations,
        [DifficultyMutation::CombatPressure {
            operation: CoreDifficultyValueOp::ScaleF32(1.5),
        }]
    );
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn sdk_runtime_difficulty_on_apply_registers_live_bus_handler() {
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
                local difficulty = require("sdk.runtime.difficulty")
                difficulty.on_apply(function(ctx)
                  ctx.combat_pressure:multiply(1.25)
                end)
                "#,
            )
            .exec()
            .expect("register live on_apply");
    }

    let event = DifficultyApplyEvent::new(DifficultySnapshot::new(
        DifficultyMode::new("treasure_log"),
        DifficultyId::new("super_hard"),
    ));
    let report = live_bus::dispatch_runtime_event(event.into());

    assert_eq!(
        report.mutations,
        [RuntimeMutation::Difficulty(
            DifficultyMutation::CombatPressure {
                operation: CoreDifficultyValueOp::ScaleF32(1.25),
            }
        )]
    );
    assert_eq!(report.errors, []);
    live_bus::reset_runtime_handlers_for_tests();
}
