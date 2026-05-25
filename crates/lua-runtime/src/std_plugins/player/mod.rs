use mlua::{Lua, Table};

use crate::{runtime::register_std_module, std_plugins::character};

const ACTIVE_CHARACTERS_GLOBAL: &str = "__oppw4_active_characters";

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let player = lua.create_table()?;
    player.set(
        "active_character",
        lua.create_function(|lua, ()| active_character(lua))?,
    )?;
    player.set(
        "active_characters",
        lua.create_function(|lua, ()| active_characters(lua))?,
    )?;
    register_std_module(lua, "player", player.clone())?;
    lua.globals().set("player", player)
}

fn active_character(lua: &Lua) -> mlua::Result<Option<Table>> {
    let characters = active_characters(lua)?;
    characters.get::<Option<Table>>(1)
}

fn active_characters(lua: &Lua) -> mlua::Result<Table> {
    let output = lua.create_table()?;
    let Some(raw_characters) = lua
        .globals()
        .get::<Option<Table>>(ACTIVE_CHARACTERS_GLOBAL)?
    else {
        return Ok(output);
    };
    let mut output_index = 1;
    for raw in raw_characters.sequence_values::<Table>() {
        let raw = raw?;
        let runtime_id = raw.get::<Option<u16>>("runtime_id")?;
        let alt_id = raw.get::<Option<u16>>("alt_id")?;
        let handle = character::active_character_handle_table(lua, runtime_id, alt_id)?;
        copy_optional_u64(&raw, &handle, "local_player")?;
        copy_optional_u64(&raw, &handle, "fx_owner")?;
        copy_optional_u64(&raw, &handle, "sequence")?;
        if let Some(alt_id) = alt_id {
            handle.set("alt_id", alt_id)?;
        }
        output.set(output_index, handle)?;
        output_index += 1;
    }
    Ok(output)
}

fn copy_optional_u64(source: &Table, target: &Table, key: &str) -> mlua::Result<()> {
    if let Some(value) = source.get::<Option<u64>>(key)? {
        target.set(key, value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Function;

    #[test]
    fn active_character_returns_known_character_handle() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        set_active_character(&lua, 1, 52, 0x1000, 0x1460, 7);

        let canonical: String = lua
            .load(
                r#"
                local player = require("std.player")
                local active = player.active_character()
                return active.canonical .. ":" .. active.runtime_id .. ":" .. active.alt_id
                "#,
            )
            .eval()
            .expect("eval");

        assert_eq!(canonical, "zoro:1:52");
    }

    #[test]
    fn active_character_handles_can_receive_character_extensions() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        crate::runtime::authorize_character_extension_owner(&lua, "sdk.runtime.fx")
            .expect("authorize");
        let module = lua.create_table().expect("module");
        module
            .set(
                "__oppw4_on_import",
                lua.create_function(|lua, ()| {
                    let register: Function =
                        lua.globals().get("__oppw4_register_character_method")?;
                    let method = lua.create_function(|_, (this, effect_id): (Table, u16)| {
                        Ok(format!("{}:{effect_id}", this.get::<String>("canonical")?))
                    })?;
                    register.call::<()>(("sdk.runtime.fx", "add_fx", method))
                })
                .expect("import"),
            )
            .expect("module hook");
        crate::runtime::register_module(&lua, "sdk.runtime.fx", module).expect("module");
        set_active_character(&lua, 1, 52, 0x1000, 0x1460, 7);

        let result: String = lua
            .load(
                r#"
                require("sdk.runtime.fx")
                local player = require("std.player")
                return player.active_character():add_fx(2830)
                "#,
            )
            .eval()
            .expect("eval");

        assert_eq!(result, "zoro:2830");
    }

    #[test]
    fn active_characters_is_empty_without_host_snapshot() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");

        let count: usize = lua
            .load(
                r#"
                local player = require("std.player")
                return #player.active_characters()
                "#,
            )
            .eval()
            .expect("eval");

        assert_eq!(count, 0);
    }

    fn set_active_character(
        lua: &Lua,
        runtime_id: u16,
        alt_id: u16,
        local_player: u64,
        fx_owner: u64,
        sequence: u64,
    ) {
        let row = lua.create_table().expect("row");
        row.set("runtime_id", runtime_id).expect("runtime");
        row.set("alt_id", alt_id).expect("alt");
        row.set("local_player", local_player).expect("local");
        row.set("fx_owner", fx_owner).expect("owner");
        row.set("sequence", sequence).expect("seq");
        let rows = lua.create_table().expect("rows");
        rows.set(1, row).expect("row set");
        lua.globals()
            .set(ACTIVE_CHARACTERS_GLOBAL, rows)
            .expect("globals");
    }
}
