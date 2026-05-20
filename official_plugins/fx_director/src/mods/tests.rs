use std::sync::{Arc, Mutex};

use mlua::{Lua, Table};

use crate::{
    config::{CycleMode, PluginConfig, TargetMode},
    mods::{
        character_ext::fx_director_module,
        state::{FxRuntimeState, SharedFxState},
    },
};

fn test_state() -> SharedFxState {
    Arc::new(Mutex::new(FxRuntimeState::new(PluginConfig::default())))
}

fn install_test_runtime(lua: &Lua) {
    lua_api::install_runtime(lua).expect("runtime");
    lua_api::authorize_character_extension_owner(lua, "fx_director").expect("authorize");
}

#[test]
fn add_fx_pushes_an_independent_effect_definition() {
    let lua = Lua::new();
    install_test_runtime(&lua);
    let state = test_state();
    let module = fx_director_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, "fx_director", module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        require("fx_director")
        local zoro = character.find("zoro")
        local fx = zoro:add_fx({
            effect_id = 2830,
            target = "local_player",
        }):timing(2.0, 1.0, 3.0)
        return fx
        "#,
    )
    .eval::<Table>()
    .expect("fx handle");

    let state = state.lock().expect("state");
    assert_eq!(state.effects.len(), 1);
    assert_eq!(state.effects[0].effect_id, 2830);
    assert_eq!(state.effects[0].target, TargetMode::LocalPlayer);
    assert_eq!(state.effects[0].animation_speed, 2.0);
}

#[test]
fn cycle_uses_fx_handles_as_presets() {
    let lua = Lua::new();
    install_test_runtime(&lua);
    let state = test_state();
    let module = fx_director_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, "fx_director", module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        local fx_director = require("fx_director")
        local zoro = character.find("zoro")
        local fx_1 = zoro:add_fx({ effect_id = 2830, target = "local_player" }):timing(2, 1, 3)
        local fx_2 = zoro:add_fx({ effect_id = 2831, target = "local_player" }):timing(1, 1, 1)
        fx_director.cycle({ fx_1, fx_2 }, { interval_ms = 750 })
        "#,
    )
    .exec()
    .expect("cycle");

    let state = state.lock().expect("state");
    assert_eq!(state.effects.len(), 2);
    assert_eq!(state.cycle.preset_count, 2);
    assert_eq!(state.cycle.presets[0].effect_id, 2830);
    assert_eq!(state.cycle.presets[0].target, TargetMode::LocalPlayer);
    assert_eq!(state.cycle.presets[0].animation_speed, 2.0);
    assert_eq!(state.cycle.presets[1].effect_id, 2831);
    assert_eq!(state.cycle.interval_ms, 750);
}

#[test]
fn cycle_can_follow_animation_timing() {
    let lua = Lua::new();
    install_test_runtime(&lua);
    let state = test_state();
    let module = fx_director_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, "fx_director", module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        local fx_director = require("fx_director")
        local zoro = character.find("zoro")
        local fx_1 = zoro:add_fx({ effect_id = 2830, target = "local_player" }):timing(1, 0.1, 1.9)
        local fx_2 = zoro:add_fx({ effect_id = 2831, target = "local_player" }):timing(1, 0.1, 1.9)
        fx_director.cycle({ fx_1, fx_2 }, { mode = "after_animation" })
        "#,
    )
    .exec()
    .expect("cycle");

    let state = state.lock().expect("state");
    assert_eq!(state.cycle.mode, CycleMode::AfterAnimation);
    assert_eq!(state.cycle.preset_count, 2);
}
