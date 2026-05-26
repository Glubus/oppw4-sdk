use mlua::Lua;

use super::lua;

fn install(lua_state: &Lua) {
    lua_api::install_runtime(lua_state).expect("runtime");
    let module = lua::module(lua_state).expect("module");
    lua_api::register_module(lua_state, lua::MODULE_NAME, module).expect("register");
}

#[test]
fn sdk_runtime_player_builds_active_character_condition() {
    let lua_state = Lua::new();
    install(&lua_state);

    let (kind, id): (String, String) = lua_state
        .load(
            r#"
            local player = require("sdk.runtime.player")
            local condition = player:active_character("zoro")
            return condition.kind, condition.id
            "#,
        )
        .eval()
        .expect("condition");

    assert_eq!(kind, "active_character");
    assert_eq!(id, "zoro");
}

#[test]
fn sdk_runtime_player_supports_active_character_builder() {
    let lua_state = Lua::new();
    install(&lua_state);

    let (kind, id): (String, String) = lua_state
        .load(
            r#"
            local player = require("sdk.runtime.player")
            local condition = player:active_character():is("zoro")
            return condition.kind, condition.id
            "#,
        )
        .eval()
        .expect("condition");

    assert_eq!(kind, "active_character");
    assert_eq!(id, "zoro");
}
