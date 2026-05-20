use mlua::{Lua, Table, Value};
use struct_api::{Character, CharacterAsset, CharacterBodyPart, CharacterCostume};

pub(super) fn character_handle_table(lua: &Lua, character: &Character) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "character")?;
    table.set("known", true)?;
    table.set("unsafe", false)?;
    if let Some(model_id) = character.model_id {
        table.set("id", model_id)?;
        table.set("model_id", model_id)?;
    }
    if let Some(playable_id) = character.playable_id {
        table.set("playable_id", playable_id)?;
    }
    if let Some(runtime_id) = character.runtime_id {
        table.set("runtime_id", runtime_id)?;
    }
    if let Some(boss_runtime_id) = character.boss_runtime_id {
        table.set("boss_runtime_id", boss_runtime_id)?;
    }
    if let Some(entry) = character.moveset_linkdata_entry {
        table.set("moveset_linkdata_entry", entry)?;
    }
    table.set("name", character.canonical.as_str())?;
    table.set("canonical", character.canonical.as_str())?;
    table.set("display_name", character.display_name.as_str())?;
    table.set("model_stem", character.model_stem.as_str())?;
    table.set("costumes", costumes_table(lua, &character.costumes)?)?;

    attach_character_metatable(lua, &table)?;
    Ok(table)
}

#[derive(Clone, Copy)]
enum UnsafeCharacterId {
    Model(u16),
    Playable(u16),
    Runtime(u16),
    BossRuntime(u16),
}

impl UnsafeCharacterId {
    fn value(self) -> u16 {
        match self {
            Self::Model(id) | Self::Playable(id) | Self::Runtime(id) | Self::BossRuntime(id) => id,
        }
    }
}

pub(super) fn unsafe_character_handle_table(lua: &Lua, query: Value) -> mlua::Result<Table> {
    let id = parse_unsafe_character_id(query)?;
    let value = id.value();
    let fields = lua.create_table()?;
    fields.set("name", format!("unsafe_{value}"))?;
    fields.set("known", false)?;
    fields.set("unsafe", true)?;
    fields.set("id", value)?;
    match id {
        UnsafeCharacterId::Model(id) => fields.set("model_id", id)?,
        UnsafeCharacterId::Playable(id) => fields.set("playable_id", id)?,
        UnsafeCharacterId::Runtime(id) => fields.set("runtime_id", id)?,
        UnsafeCharacterId::BossRuntime(id) => fields.set("boss_runtime_id", id)?,
    }
    custom_character_handle_table(lua, fields)
}

pub(super) fn custom_character_handle_table(lua: &Lua, fields: Table) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    let id = first_u16_field(
        &fields,
        &[
            "id",
            "model_id",
            "runtime_id",
            "playable_id",
            "boss_runtime_id",
        ],
    )?;
    let fallback_name = id
        .map(|id| format!("custom_{id}"))
        .unwrap_or_else(|| "custom_character".to_string());

    table.set("kind", "character")?;
    table.set(
        "known",
        fields.get::<Option<bool>>("known")?.unwrap_or(false),
    )?;
    table.set(
        "unsafe",
        fields.get::<Option<bool>>("unsafe")?.unwrap_or(false),
    )?;
    copy_optional_u16(&fields, &table, "id")?;
    copy_optional_u16(&fields, &table, "model_id")?;
    copy_optional_u16(&fields, &table, "playable_id")?;
    copy_optional_u16(&fields, &table, "runtime_id")?;
    copy_optional_u16(&fields, &table, "boss_runtime_id")?;
    copy_optional_u16(&fields, &table, "moveset_linkdata_entry")?;
    if table.get::<Option<u16>>("id")?.is_none() {
        if let Some(id) = id {
            table.set("id", id)?;
        }
    }
    let name = fields
        .get::<Option<String>>("name")?
        .or_else(|| fields.get::<Option<String>>("canonical").ok().flatten())
        .unwrap_or(fallback_name);
    let canonical = fields
        .get::<Option<String>>("canonical")?
        .unwrap_or_else(|| name.clone());
    table.set("name", name.as_str())?;
    table.set("canonical", canonical.as_str())?;
    table.set(
        "display_name",
        fields
            .get::<Option<String>>("display_name")?
            .unwrap_or_else(|| name.clone()),
    )?;
    table.set(
        "model_stem",
        fields
            .get::<Option<String>>("model_stem")?
            .unwrap_or_else(|| id.map(|id| format!("MPLC{id:03}")).unwrap_or_default()),
    )?;

    attach_character_metatable(lua, &table)?;
    Ok(table)
}

fn costumes_table(lua: &Lua, costumes: &[CharacterCostume]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, costume) in costumes.iter().enumerate() {
        let costume_table = costume_table(lua, costume)?;
        table.set(index + 1, costume_table.clone())?;
        table.set(costume.id.as_str(), costume_table)?;
    }
    Ok(table)
}

fn costume_table(lua: &Lua, costume: &CharacterCostume) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", costume.id.as_str())?;
    table.set("label", costume.label.as_str())?;
    if let Some(model_id) = costume.model_id {
        table.set("model_id", model_id)?;
    }
    table.set("assets", assets_table(lua, &costume.assets)?)?;
    table.set("body_parts", body_parts_table(lua, &costume.body_parts)?)?;
    Ok(table)
}

fn body_parts_table(lua: &Lua, body_parts: &[CharacterBodyPart]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, part) in body_parts.iter().enumerate() {
        let part_table = lua.create_table()?;
        part_table.set("id", part.id.as_str())?;
        part_table.set("label", part.label.as_str())?;
        part_table.set("assets", assets_table(lua, &part.assets)?)?;
        table.set(index + 1, part_table.clone())?;
        table.set(part.id.as_str(), part_table)?;
    }
    Ok(table)
}

fn assets_table(lua: &Lua, assets: &[CharacterAsset]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, asset) in assets.iter().enumerate() {
        let asset_table = lua.create_table()?;
        asset_table.set("kind", asset.kind.as_str())?;
        asset_table.set("label", asset.label.as_str())?;
        set_optional_string(&asset_table, "variant", asset.variant.as_deref())?;
        set_optional_string(&asset_table, "archive", asset.archive.as_deref())?;
        set_optional_string(&asset_table, "path", asset.path.as_deref())?;
        set_optional_string(&asset_table, "hash", asset.hash.as_deref())?;
        set_optional_string(&asset_table, "file_type", asset.file_type.as_deref())?;
        table.set(index + 1, asset_table)?;
    }
    Ok(table)
}

fn set_optional_string(table: &Table, key: &str, value: Option<&str>) -> mlua::Result<()> {
    if let Some(value) = value {
        table.set(key, value)?;
    }
    Ok(())
}

pub(super) fn local_player_handle_table(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "local_player")?;
    table.set("id", -1)?;
    table.set("name", "local_player")?;
    table.set("canonical", "local_player")?;
    table.set("display_name", "Local Player")?;

    attach_character_metatable(lua, &table)?;
    Ok(table)
}

fn attach_character_metatable(lua: &Lua, table: &Table) -> mlua::Result<()> {
    let metatable = lua.create_table()?;
    let methods: Table = lua.globals().get("__struct_api_methods")?;
    metatable.set(
        "__index",
        lua.create_function(move |_, (_this, key): (Table, String)| {
            methods.get::<Value>(key.as_str())
        })?,
    )?;
    table.set_metatable(Some(metatable));
    Ok(())
}

fn first_u16_field(fields: &Table, keys: &[&str]) -> mlua::Result<Option<u16>> {
    for key in keys {
        if let Some(value) = fields.get::<Option<u16>>(*key)? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn copy_optional_u16(source: &Table, target: &Table, key: &str) -> mlua::Result<()> {
    if let Some(value) = source.get::<Option<u16>>(key)? {
        target.set(key, value)?;
    }
    Ok(())
}

fn parse_unsafe_character_id(query: Value) -> mlua::Result<UnsafeCharacterId> {
    match query {
        Value::Integer(id) if (0..=u16::MAX as i64).contains(&id) => {
            Ok(UnsafeCharacterId::Model(id as u16))
        }
        Value::Table(table) => {
            if let Some(id) = table.get::<Option<u16>>("model_id")? {
                return Ok(UnsafeCharacterId::Model(id));
            }
            if let Some(id) = table.get::<Option<u16>>("id")? {
                return Ok(UnsafeCharacterId::Model(id));
            }
            if let Some(id) = table.get::<Option<u16>>("playable_id")? {
                return Ok(UnsafeCharacterId::Playable(id));
            }
            if let Some(id) = table.get::<Option<u16>>("runtime_id")? {
                return Ok(UnsafeCharacterId::Runtime(id));
            }
            if let Some(id) = table.get::<Option<u16>>("boss_runtime_id")? {
                return Ok(UnsafeCharacterId::BossRuntime(id));
            }
            Err(mlua::Error::external(
                "character.unsafe_find expects an id, model_id, playable_id, runtime_id, or boss_runtime_id",
            ))
        }
        _ => Err(mlua::Error::external(
            "character.unsafe_find expects a numeric id or id table",
        )),
    }
}
