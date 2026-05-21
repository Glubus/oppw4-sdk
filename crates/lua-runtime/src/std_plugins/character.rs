use mlua::{Lua, Table, Value};

use crate::runtime::{register_module, register_std_module};

mod extensions;
mod handles;

use handles::{
    character_handle_table, custom_character_handle_table, local_player_handle_table,
    unsafe_character_handle_table,
};

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    extensions::install_registry(lua)?;

    let character = lua.create_table()?;
    character.set(
        "find",
        lua.create_function(|lua, query: Value| match query {
            Value::String(name) => {
                let Some(character) = struct_api::find(name.to_str()?.as_ref()) else {
                    return Ok(Value::Nil);
                };
                Ok(Value::Table(character_handle_table(lua, character)?))
            }
            Value::Integer(id) if (0..=u16::MAX as i64).contains(&id) => {
                let Some(character) = struct_api::find_by_id(id as u16) else {
                    return Ok(Value::Nil);
                };
                Ok(Value::Table(character_handle_table(lua, character)?))
            }
            _ => Ok(Value::Nil),
        })?,
    )?;
    character.set(
        "unsafe_find",
        lua.create_function(|lua, query: Value| unsafe_character_handle_table(lua, query))?,
    )?;
    character.set(
        "new",
        lua.create_function(|lua, fields: Table| custom_character_handle_table(lua, fields))?,
    )?;
    character.set(
        "all",
        lua.create_function(|lua, ()| {
            let rows = lua.create_table()?;
            for (index, character) in struct_api::all().iter().enumerate() {
                rows.set(index + 1, character_handle_table(lua, character)?)?;
            }
            Ok(rows)
        })?,
    )?;
    character.set(
        "local_player",
        lua.create_function(|lua, ()| local_player_handle_table(lua))?,
    )?;
    register_std_module(lua, "character", character.clone())?;
    register_module(lua, "character", character.clone())?;
    lua.globals().set("character", character)
}

pub(crate) fn authorize_extension_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    extensions::authorize_owner(lua, owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Function;

    #[test]
    fn required_plugin_can_extend_character_handles_once() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        authorize_extension_owner(&lua, "skin_patcher").expect("authorize");
        let patcher = lua.create_table().expect("patcher");
        patcher
            .set(
                "__oppw4_on_import",
                lua.create_function(|lua, ()| {
                    let register: Function =
                        lua.globals().get("__oppw4_register_character_method")?;
                    let method =
                        lua.create_function(|_, (_this, slot, file): (Table, u16, String)| {
                            Ok(format!("{slot}:{file}"))
                        })?;
                    register.call::<()>(("skin_patcher", "replace_costume", method))
                })
                .expect("on import"),
            )
            .expect("hook");
        register_module(&lua, "skin_patcher", patcher).expect("module");

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
                require("skin_patcher")
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
        crate::runtime::install_runtime(&lua).expect("runtime");

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
        crate::runtime::install_runtime(&lua).expect("runtime");

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
        crate::runtime::install_runtime(&lua).expect("runtime");

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
        crate::runtime::install_runtime(&lua).expect("runtime");

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
        crate::runtime::install_runtime(&lua).expect("runtime");

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
        crate::runtime::install_runtime(&lua).expect("runtime");
        authorize_extension_owner(&lua, "fx_director").expect("authorize");
        let patcher = lua.create_table().expect("patcher");
        patcher
            .set(
                "__oppw4_on_import",
                lua.create_function(|lua, ()| {
                    let register: Function =
                        lua.globals().get("__oppw4_register_character_method")?;
                    let method = lua.create_function(|_, (_this, effect): (Table, u16)| {
                        Ok(format!("fx:{effect}"))
                    })?;
                    register.call::<()>(("fx_director", "add_fx", method))
                })
                .expect("on import"),
            )
            .expect("hook");
        register_module(&lua, "fx_director", patcher).expect("module");

        let value: String = lua
            .load(
                r#"
                require("fx_director")
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
        crate::runtime::install_runtime(&lua).expect("runtime");
        authorize_extension_owner(&lua, "skin_patcher").expect("authorize skin");
        authorize_extension_owner(&lua, "other_plugin").expect("authorize other");

        let register: Function = lua
            .globals()
            .get("__oppw4_register_character_method")
            .expect("register");
        let first = lua.create_function(|_, ()| Ok(())).expect("first");
        let second = lua.create_function(|_, ()| Ok(())).expect("second");

        register
            .call::<()>(("skin_patcher", "replace_costume", first))
            .expect("first register");
        let error = register
            .call::<()>(("other_plugin", "replace_costume", second))
            .expect_err("conflict");

        assert!(error
            .to_string()
            .contains("already registered by skin_patcher"));
    }

    #[test]
    fn character_extension_requires_authorized_owner() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");

        let register: Function = lua
            .globals()
            .get("__oppw4_register_character_method")
            .expect("register");
        let method = lua.create_function(|_, ()| Ok(())).expect("method");
        let error = register
            .call::<()>(("skin_patcher", "replace_costume", method))
            .expect_err("unauthorized extension");

        assert!(error.to_string().contains("missing std.character.extend"));
    }
}
