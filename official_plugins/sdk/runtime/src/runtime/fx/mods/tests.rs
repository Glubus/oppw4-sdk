use std::sync::{Arc, Mutex};

use mlua::{Function, Lua, Table};

use crate::runtime::fx::{
    config::{CycleMode, PluginConfig, TargetMode},
    mods::{
        character_ext::runtime_fx_module,
        state::{FxRuntimeState, SharedFxState},
    },
};

const MODULE_NAME: &str = "sdk.runtime.fx";

fn test_state() -> SharedFxState {
    Arc::new(Mutex::new(FxRuntimeState::new(PluginConfig::default())))
}

fn install_test_runtime(lua: &Lua) {
    lua_api::install_runtime(lua).expect("runtime");
    install_test_character_module(lua);
    lua_api::authorize_character_extension_owner(lua, MODULE_NAME).expect("authorize");
}

fn install_test_character_module(lua: &Lua) {
    let authorized = lua.create_table().expect("authorized owners");
    lua.globals()
        .set("__struct_api_authorized_method_owners", authorized.clone())
        .expect("authorized global");

    let zoro = lua.create_table().expect("zoro");
    zoro.set("canonical", "zoro").expect("canonical");
    zoro.set("name", "zoro").expect("name");
    zoro.set("runtime_id", 1u16).expect("runtime_id");
    zoro.set("boss_runtime_id", 1u16).expect("boss_runtime_id");
    zoro.set("playable_id", 1u16).expect("playable_id");
    zoro.set("model_id", 1u16).expect("model_id");

    let extension_target = zoro.clone();
    lua.globals()
        .set(
            "__oppw4_register_character_method",
            lua.create_function(
                move |_, (owner, name, method): (String, String, Function)| {
                    let allowed = authorized
                        .get::<Option<bool>>(owner.to_ascii_lowercase())?
                        .unwrap_or(false);
                    if !allowed {
                        return Err(mlua::Error::external(format!(
                            "character.{name} refused for {owner}: missing std.character.extend"
                        )));
                    }
                    extension_target.set(name, method)
                },
            )
            .expect("register function"),
        )
        .expect("register global");

    let character = lua.create_table().expect("character");
    character
        .set(
            "find",
            lua.create_function(move |_, name: String| {
                if name == "zoro" {
                    Ok(Some(zoro.clone()))
                } else {
                    Ok(None)
                }
            })
            .expect("find"),
        )
        .expect("find");
    lua_api::register_module(lua, "std.character", character).expect("std.character");
}

#[test]
fn add_fx_pushes_an_independent_effect_definition() {
    let lua = Lua::new();
    install_test_runtime(&lua);
    let state = test_state();
    let module = runtime_fx_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, MODULE_NAME, module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        require("sdk.runtime.fx")
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
    let module = runtime_fx_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, MODULE_NAME, module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        local fx = require("sdk.runtime.fx")
        local zoro = character.find("zoro")
        local fx_1 = zoro:add_fx({ effect_id = 2830, target = "local_player" }):timing(2, 1, 3)
        local fx_2 = zoro:add_fx({ effect_id = 2831, target = "local_player" }):timing(1, 1, 1)
        fx.cycle({ fx_1, fx_2 }, { interval_ms = 750 })
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
    let module = runtime_fx_module(&lua, Arc::clone(&state)).expect("module");
    lua_api::register_module(&lua, MODULE_NAME, module).expect("register");

    lua.load(
        r#"
        local character = require("std.character")
        local fx = require("sdk.runtime.fx")
        local zoro = character.find("zoro")
        local fx_1 = zoro:add_fx({ effect_id = 2830, target = "local_player" }):timing(1, 0.1, 1.9)
        local fx_2 = zoro:add_fx({ effect_id = 2831, target = "local_player" }):timing(1, 0.1, 1.9)
        fx.cycle({ fx_1, fx_2 }, { mode = "after_animation" })
        "#,
    )
    .exec()
    .expect("cycle");

    let state = state.lock().expect("state");
    assert_eq!(state.cycle.mode, CycleMode::AfterAnimation);
    assert_eq!(state.cycle.preset_count, 2);
}
