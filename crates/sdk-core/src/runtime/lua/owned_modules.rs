use mlua::{Function, Lua, Table};

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    ensure_pending_mutations(lua)?;
    install_character_module(lua)?;
    install_moveset_patcher_module(lua)
}

fn ensure_pending_mutations(lua: &Lua) -> mlua::Result<()> {
    if lua
        .globals()
        .get::<Option<Table>>("__oppw4_pending_mutations")?
        .is_none()
    {
        lua.globals()
            .set("__oppw4_pending_mutations", lua.create_table()?)?;
    }
    Ok(())
}

fn install_character_module(lua: &Lua) -> mlua::Result<()> {
    let by_key = lua.create_table()?;
    let replace_movesets = replace_movesets_function(lua)?;

    for character in struct_api::all() {
        let handle = lua.create_table()?;
        handle.set("kind", "character")?;
        handle.set("known", true)?;
        handle.set("unsafe", false)?;
        handle.set("name", character.canonical.as_str())?;
        handle.set("canonical", character.canonical.as_str())?;
        handle.set("display_name", character.display_name.as_str())?;
        handle.set("model_stem", character.model_stem.as_str())?;
        if let Some(model_id) = character.model_id {
            handle.set("id", model_id)?;
            handle.set("model_id", model_id)?;
            by_key.set(model_id, handle.clone())?;
        }
        if let Some(playable_id) = character.playable_id {
            handle.set("playable_id", playable_id)?;
            by_key.set(playable_id, handle.clone())?;
        }
        if let Some(runtime_id) = character.runtime_id {
            handle.set("runtime_id", runtime_id)?;
        }
        if let Some(boss_runtime_id) = character.boss_runtime_id {
            handle.set("boss_runtime_id", boss_runtime_id)?;
        }
        if let Some(entry) = character.moveset_linkdata_entry {
            handle.set("moveset_linkdata_entry", entry)?;
        }
        handle.set("replace_movesets", replace_movesets.clone())?;

        by_key.set(character.canonical.to_ascii_lowercase(), handle.clone())?;
        by_key.set(character.canonical.as_str(), handle.clone())?;
        for alias in &character.aliases {
            by_key.set(alias.to_ascii_lowercase(), handle.clone())?;
            by_key.set(alias.as_str(), handle.clone())?;
        }
    }

    let character = lua.create_table()?;
    character.set("__by_key", by_key.clone())?;
    character.set(
        "find",
        lua.load(
            r#"
            local by_key = ...
            return function(query)
                if type(query) == "string" then
                    return by_key[string.lower(query)] or by_key[query]
                end
                return by_key[query]
            end
            "#,
        )
        .call::<Function>(by_key)?,
    )?;

    register_module(lua, "std.character", character.clone())?;
    register_module(lua, "character", character.clone())?;
    let globals = lua.globals();
    let std = match globals.get::<Option<Table>>("std")? {
        Some(std) => std,
        None => lua.create_table()?,
    };
    std.set("character", character.clone())?;
    globals.set("std", std)?;
    globals.set("character", character)
}

fn replace_movesets_function(lua: &Lua) -> mlua::Result<Function> {
    lua.load(
        r#"
        return function(character, moveset)
            local entry = character.moveset_linkdata_entry
            if entry == nil then
                local name = character.canonical or character.name or "unknown"
                error("no SDK moveset target for character=" .. tostring(name))
            end
            local queue = rawget(_G, "__oppw4_pending_mutations")
            if queue == nil then
                queue = {}
                rawset(_G, "__oppw4_pending_mutations", queue)
            end
            local mutation = {
                type = "moveset.replace",
                mod_id = rawget(_G, "__oppw4_mod_id") or "",
                character = character.canonical or character.name,
                entry = entry,
                payload_file = moveset.payload_file,
                payload = moveset.payload,
            }
            queue[#queue + 1] = mutation
            local trace = rawget(_G, "__oppw4_trace")
            if trace ~= nil then
                trace("moveset mutation queued character=" .. tostring(mutation.character) .. " entry=" .. tostring(entry) .. " payload_file=" .. tostring(mutation.payload_file))
            end
        end
        "#,
    )
    .eval()
}

fn install_moveset_patcher_module(lua: &Lua) -> mlua::Result<()> {
    let module = lua.create_table()?;
    module.set("id", "moveset_patcher")?;
    module.set(
        "patch",
        lua.load(
            r#"
            return function(definition)
                return definition
            end
            "#,
        )
        .eval::<Function>()?,
    )?;
    register_module(lua, "moveset_patcher", module)
}

fn register_module(lua: &Lua, name: &str, module: Table) -> mlua::Result<()> {
    lua_api::register_module(lua, name, module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_find_and_moveset_patch_queue_mutation_without_rust_callbacks() {
        let lua = Lua::new();
        lua_api::install_runtime(&lua).expect("runtime");
        install(&lua).expect("owned modules");
        lua.globals()
            .set("__oppw4_mod_id", "ace_test")
            .expect("mod id");

        let queued: (String, u16, String) = lua
            .load(
                r#"
                local moveset_patcher = require("moveset_patcher")
                local character = require("std.character")
                local ace = character.find("ace")
                local moveset = moveset_patcher.patch({ payload_file = "ace_moveset.bin" })
                ace:replace_movesets(moveset)
                local mutation = __oppw4_pending_mutations[1]
                return mutation.character, mutation.entry, mutation.payload_file
                "#,
            )
            .eval()
            .expect("mutation queued");

        assert_eq!(queued.0, "ace");
        assert_eq!(queued.2, "ace_moveset.bin");
        assert!(queued.1 > 0);
    }
}
