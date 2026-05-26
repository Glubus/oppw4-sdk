use mlua::Lua;

use crate::runtime::core::{
    live_bus,
    player::{PlayerChangeEvent, PlayerSnapshot},
};

use super::lua;

fn install_with_registry(lua_state: &Lua) -> lua::PlayerLuaRegistry {
    lua_api::install_runtime(lua_state).expect("runtime");
    let registry = lua::PlayerLuaRegistry::new();
    let module = lua::module_with_registry(lua_state, registry.clone()).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
    registry
}

#[test]
fn sdk_runtime_player_exposes_only_event_api() {
    let lua_state = Lua::new();
    install_with_registry(&lua_state);

    let values: (String, String) = lua_state
        .load(
            r#"
            local player = require("sdk.runtime.player")
            return type(player.on_change), tostring(player.active_character)
            "#,
        )
        .eval()
        .expect("api shape");

    assert_eq!(values, ("function".to_string(), "nil".to_string()));
}

#[test]
fn sdk_runtime_player_on_change_receives_core_snapshot() {
    let lua_state = Lua::new();
    let registry = install_with_registry(&lua_state);
    lua_state
        .load(
            r#"
            local player = require("sdk.runtime.player")
            seen = false
            player.on_change(function(ctx)
              seen = ctx:has_active_character("zoro")
            end)
            "#,
        )
        .exec()
        .expect("register on_change");

    let report = registry.dispatch_player_change(
        &lua_state,
        &PlayerChangeEvent::new(PlayerSnapshot::new().with_active_character("zoro")),
    );
    let seen: bool = lua_state.globals().get("seen").expect("seen");

    assert_eq!(registry.len(), 1);
    assert!(seen);
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn sdk_runtime_player_on_change_registers_live_bus_handler() {
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
                local player = require("sdk.runtime.player")
                player.on_change(function(ctx)
                  if ctx:has_active_character("zoro") then
                    error("live player callback fired")
                  end
                end)
                "#,
            )
            .exec()
            .expect("register live on_change");
    }

    let report = live_bus::dispatch_runtime_event(
        PlayerChangeEvent::new(PlayerSnapshot::new().with_active_character("zoro")).into(),
    );

    assert_eq!(report.mutations, []);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].error.message().contains("fired"));
    live_bus::reset_runtime_handlers_for_tests();
}
