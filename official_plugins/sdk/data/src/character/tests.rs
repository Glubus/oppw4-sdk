use mlua::{Function, Lua, Table};

use super::authorize_extension_owner;

fn install_character(lua: &Lua) {
    lua_api::install_require_hook(lua).expect("require hook");
    super::install(lua).expect("character");
}

#[test]
fn required_plugin_can_extend_character_handles_once() {
    let lua = Lua::new();
    install_character(&lua);
    authorize_extension_owner(&lua, "sdk.rdb.patcher").expect("authorize");
    let patcher = lua.create_table().expect("patcher");
    patcher
        .set(
            "__oppw4_on_import",
            lua.create_function(|lua, ()| {
                let register: Function = lua.globals().get("__oppw4_register_character_method")?;
                let method =
                    lua.create_function(|_, (_this, slot, file): (Table, u16, String)| {
                        Ok(format!("{slot}:{file}"))
                    })?;
                register.call::<()>(("sdk.rdb.patcher", "replace_costume", method))
            })
            .expect("on import"),
        )
        .expect("hook");
    lua_api::register_module(&lua, "sdk.rdb.patcher", patcher).expect("module");

    let before: bool = lua
        .load(
            r#"
            local law = character.find("law")
            return law.replace_costume == nil
            "#,
        )
        .eval()
        .expect("before");
    assert!(before);

    let value: String = lua
        .load(
            r#"
            require("sdk.rdb.patcher")
            local law = character.find("law")
            return law:replace_costume(3, "my_model.g1m")
            "#,
        )
        .eval()
        .expect("after");

    assert_eq!(value, "3:my_model.g1m");
}

#[test]
fn std_character_module_is_requireable() {
    let lua = Lua::new();
    install_character(&lua);

    let value: String = lua
        .load(
            r#"
            local character = require("std.character")
            local law = character.find("law")
            return law.canonical
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "law");
}

#[test]
fn std_global_exposes_character_module() {
    let lua = Lua::new();
    install_character(&lua);

    let value: String = lua
        .load(
            r#"
            local law = std.character.find("law")
            return law.canonical
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "law");
}

#[test]
fn character_find_accepts_known_ids() {
    let lua = Lua::new();
    install_character(&lua);

    let value: String = lua
        .load(
            r#"
            local law_by_model = character.find(26)
            local law_by_playable = character.find(22)
            return law_by_model.name .. ":" .. law_by_playable.name
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "law:law");
}

#[test]
fn character_unsafe_find_returns_unchecked_model_handle() {
    let lua = Lua::new();
    install_character(&lua);

    let value: String = lua
        .load(
            r#"
            local custom = character.unsafe_find(730)
            return tostring(custom.known) .. ":" .. tostring(custom.unsafe) .. ":" .. custom.name .. ":" .. custom.model_id
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "false:true:unsafe_730:730");
}

#[test]
fn character_unsafe_find_preserves_requested_id_kind() {
    let lua = Lua::new();
    install_character(&lua);

    let value: String = lua
        .load(
            r#"
            local runtime = character.unsafe_find({ runtime_id = 730 })
            local playable = character.unsafe_find({ playable_id = 44 })
            return tostring(runtime.model_id) .. ":" .. runtime.runtime_id .. ":" .. tostring(playable.runtime_id) .. ":" .. playable.playable_id
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "nil:730:nil:44");
}

#[test]
fn character_new_creates_custom_handles_with_plugin_methods() {
    let lua = Lua::new();
    install_character(&lua);
    authorize_extension_owner(&lua, "sdk.runtime.fx").expect("authorize");
    let patcher = lua.create_table().expect("patcher");
    patcher
        .set(
            "__oppw4_on_import",
            lua.create_function(|lua, ()| {
                let register: Function = lua.globals().get("__oppw4_register_character_method")?;
                let method = lua.create_function(|_, (_this, effect): (Table, u16)| {
                    Ok(format!("fx:{effect}"))
                })?;
                register.call::<()>(("sdk.runtime.fx", "add_fx", method))
            })
            .expect("on import"),
        )
        .expect("hook");
    lua_api::register_module(&lua, "sdk.runtime.fx", patcher).expect("module");

    let value: String = lua
        .load(
            r#"
            require("sdk.runtime.fx")
            local custom = character.new({
                name = "my_custom_zoro",
                runtime_id = 730,
                model_stem = "CUSTOM_Zoro",
            })
            return custom.name .. ":" .. tostring(custom.model_id) .. ":" .. custom.runtime_id .. ":" .. custom:add_fx(2830)
            "#,
        )
        .eval()
        .expect("eval");

    assert_eq!(value, "my_custom_zoro:nil:730:fx:2830");
}

#[test]
fn character_extension_method_conflicts_fail_loudly() {
    let lua = Lua::new();
    install_character(&lua);
    authorize_extension_owner(&lua, "sdk.rdb.patcher").expect("authorize skin");
    authorize_extension_owner(&lua, "other_plugin").expect("authorize other");

    let register: Function = lua
        .globals()
        .get("__oppw4_register_character_method")
        .expect("register");
    let first = lua.create_function(|_, ()| Ok(())).expect("first");
    let second = lua.create_function(|_, ()| Ok(())).expect("second");

    register
        .call::<()>(("sdk.rdb.patcher", "replace_costume", first))
        .expect("first register");
    let error = register
        .call::<()>(("other_plugin", "replace_costume", second))
        .expect_err("conflict");

    assert!(error
        .to_string()
        .contains("already registered by sdk.rdb.patcher"));
}

#[test]
fn character_extension_requires_authorized_owner() {
    let lua = Lua::new();
    install_character(&lua);

    let register: Function = lua
        .globals()
        .get("__oppw4_register_character_method")
        .expect("register");
    let method = lua.create_function(|_, ()| Ok(())).expect("method");
    let error = register
        .call::<()>(("sdk.rdb.patcher", "replace_costume", method))
        .expect_err("unauthorized extension");

    assert!(error.to_string().contains("missing std.character.extend"));
}
