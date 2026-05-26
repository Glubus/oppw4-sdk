use std::{ffi::c_void, path::Path};

use mlua::{Function, Lua, Table, Value};
use plugin_sdk::{HostApi, PluginError};

use crate::{
    log,
    state::{self, AssetReplacement},
};

const CHARACTER_ARCHIVE: &str = "CharacterEditor";
const MODULE_OWNER: &str = "sdk_rdb";
const MODULE_NAME: &str = "sdk.rdb.patcher";

pub fn register(host: HostApi<'_>) {
    let result = match host.lua().register_module_fn(
        MODULE_OWNER,
        MODULE_NAME,
        std::ptr::null_mut(),
        register_rdb_patcher_module,
    ) {
        Ok(()) => 0,
        Err(PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };
    if result != 0 {
        log::write_line(format!(
            "sdk.rdb.patcher lua module register failed result={result}"
        ));
    }
}

unsafe extern "system" fn register_rdb_patcher_module(
    _context: *mut c_void,
    lua: *mut c_void,
) -> i32 {
    let Some(lua) = lua.cast::<Lua>().as_ref() else {
        return -1;
    };
    match rdb_patcher_module(lua)
        .and_then(|table| lua_api::register_module(lua, MODULE_NAME, table))
    {
        Ok(()) => 0,
        Err(error) => {
            log::write_line(format!("sdk.rdb.patcher lua module failed: {error}"));
            -2
        }
    }
}

fn rdb_patcher_module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", MODULE_NAME)?;
    table.set(
        "__oppw4_on_import",
        lua.create_function(|lua, ()| register_character_extensions(lua))?,
    )?;
    Ok(table)
}

fn register_character_extensions(lua: &Lua) -> mlua::Result<()> {
    let register: Function = lua.globals().get("__oppw4_register_character_method")?;
    register.call::<()>((
        MODULE_NAME,
        "replace_costume",
        lua.create_function(replace_costume)?,
    ))?;
    register.call::<()>((
        MODULE_NAME,
        "replace_portrait",
        lua.create_function(replace_portrait)?,
    ))?;
    register.call::<()>((
        MODULE_NAME,
        "replace_model",
        lua.create_function(replace_model)?,
    ))?;
    register.call::<()>((
        MODULE_NAME,
        "replace_textures",
        lua.create_function(replace_textures)?,
    ))
}

fn replace_costume(_: &Lua, (character, slot, model): (Table, u16, String)) -> mlua::Result<()> {
    if slot == 0 {
        return Err(mlua::Error::external(
            "replace_costume slot is 1-based; use 1 for the first costume",
        ));
    }
    let name = character_name(&character)?;
    let model_id = character.get::<u16>("model_id").ok();
    log::write_line(format!(
        "lua skin_patcher replace_costume character={name} model_id={model_id:?} slot={slot} model={model}"
    ));
    Ok(())
}

fn replace_portrait(
    _: &Lua,
    (character, slot, portrait): (Table, u16, String),
) -> mlua::Result<()> {
    if slot == 0 {
        return Err(mlua::Error::external(
            "replace_portrait slot is 1-based; use 1 for the first portrait",
        ));
    }
    let name = character_name(&character)?;
    let model_id = character.get::<u16>("model_id").ok();
    log::write_line(format!(
        "lua skin_patcher replace_portrait character={name} model_id={model_id:?} slot={slot} portrait={portrait}"
    ));
    Ok(())
}

fn replace_model(
    lua: &Lua,
    (character, costume, model): (Table, String, String),
) -> mlua::Result<()> {
    let name = character_name(&character)?;
    let target = model_target_file(&character, &costume)?;
    let source = state::mod_source_from_lua(lua, Path::new(&model))?;
    let count = state::register_asset_replacements(
        CHARACTER_ARCHIVE,
        vec![AssetReplacement {
            target_file_name: target.clone(),
            source,
        }],
    )
    .map_err(mlua::Error::external)?;
    log::write_line(format!(
        "lua skin_patcher replace_model character={name} costume={costume} target={target} source={model} replacements={count}"
    ));
    Ok(())
}

fn replace_textures(lua: &Lua, args: mlua::MultiValue) -> mlua::Result<()> {
    let (character, costume, texture_args) = parse_texture_args(args)?;
    let name = character_name(&character)?;
    let mut replacements = Vec::with_capacity(texture_args.len());
    for (target, source_path) in texture_args {
        let target = texture_target_file(&character, &costume, &target)?;
        replacements.push(AssetReplacement {
            target_file_name: target,
            source: state::mod_source_from_lua(lua, Path::new(&source_path))?,
        });
    }
    let count = state::register_asset_replacements(CHARACTER_ARCHIVE, replacements)
        .map_err(mlua::Error::external)?;
    log::write_line(format!(
        "lua skin_patcher replace_textures character={name} costume={costume} replacements={count}"
    ));
    Ok(())
}

fn character_name(character: &Table) -> mlua::Result<String> {
    character
        .get::<Option<String>>("canonical")?
        .or_else(|| character.get::<Option<String>>("name").ok().flatten())
        .ok_or_else(|| mlua::Error::external("skin_patcher method called without a character"))
}

fn model_target_file(character: &Table, costume: &str) -> mlua::Result<String> {
    if let Some(path) = costume_asset_path(character, costume, "model")? {
        return Ok(ensure_extension(&path, "g1m"));
    }
    let stem = character
        .get::<Option<String>>("model_stem")?
        .or_else(|| {
            character
                .get::<Option<u16>>("model_id")
                .ok()
                .flatten()
                .map(|id| format!("MPLC{id:03}"))
        })
        .ok_or_else(|| {
            mlua::Error::external("replace_model needs a character with model_stem or model_id")
        })?;
    Ok(ensure_extension(&stem, "g1m"))
}

fn texture_target_file(
    character: &Table,
    costume: &str,
    part_or_file: &str,
) -> mlua::Result<String> {
    if let Some(path) = body_part_asset_path(character, costume, part_or_file, "texture")? {
        return Ok(ensure_extension(&path, "g1t"));
    }
    Ok(ensure_extension(part_or_file, "g1t"))
}

fn costume_asset_path(
    character: &Table,
    costume: &str,
    kind: &str,
) -> mlua::Result<Option<String>> {
    let Some(costume) = costume_table(character, costume)? else {
        return Ok(None);
    };
    asset_path(costume.get::<Option<Table>>("assets")?, kind)
}

fn body_part_asset_path(
    character: &Table,
    costume: &str,
    body_part: &str,
    kind: &str,
) -> mlua::Result<Option<String>> {
    let Some(costume) = costume_table(character, costume)? else {
        return Ok(None);
    };
    let Some(body_parts) = costume.get::<Option<Table>>("body_parts")? else {
        return Ok(None);
    };
    let Some(part) = body_parts.get::<Option<Table>>(body_part)? else {
        return Ok(None);
    };
    asset_path(part.get::<Option<Table>>("assets")?, kind)
}

fn costume_table(character: &Table, costume: &str) -> mlua::Result<Option<Table>> {
    let Some(costumes) = character.get::<Option<Table>>("costumes")? else {
        return Ok(None);
    };
    costumes.get::<Option<Table>>(costume)
}

fn asset_path(assets: Option<Table>, kind: &str) -> mlua::Result<Option<String>> {
    let Some(assets) = assets else {
        return Ok(None);
    };
    for asset in assets.sequence_values::<Table>() {
        let asset = asset?;
        if asset.get::<Option<String>>("kind")?.as_deref() == Some(kind) {
            if let Some(path) = asset.get::<Option<String>>("path")? {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

fn parse_texture_args(
    args: mlua::MultiValue,
) -> mlua::Result<(Table, String, Vec<(String, String)>)> {
    let mut args = args.into_iter();
    let character = match args.next() {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(mlua::Error::external(
                "replace_textures expects character:replace_textures(costume, ...)",
            ))
        }
    };
    let costume = match args.next() {
        Some(Value::String(value)) => value.to_str()?.to_string(),
        _ => {
            return Err(mlua::Error::external(
                "replace_textures expects a costume name as first argument",
            ))
        }
    };
    let Some(next) = args.next() else {
        return Err(mlua::Error::external(
            "replace_textures expects a texture target or table",
        ));
    };
    let replacements = match next {
        Value::String(texture) => {
            let Some(Value::String(source)) = args.next() else {
                return Err(mlua::Error::external(
                    "replace_textures expects texture target and source file",
                ));
            };
            vec![(texture.to_str()?.to_string(), source.to_str()?.to_string())]
        }
        Value::Table(table) => texture_pairs(table)?,
        _ => {
            return Err(mlua::Error::external(
                "replace_textures expects a texture target string or array table",
            ))
        }
    };
    Ok((character, costume, replacements))
}

fn texture_pairs(table: Table) -> mlua::Result<Vec<(String, String)>> {
    let mut pairs = Vec::new();
    for value in table.sequence_values::<Table>() {
        let pair = value?;
        pairs.push((pair.get::<String>(1)?, pair.get::<String>(2)?));
    }
    if pairs.is_empty() {
        return Err(mlua::Error::external(
            "replace_textures table must contain {target, source_file} pairs",
        ));
    }
    Ok(pairs)
}

fn ensure_extension(value: &str, extension: &str) -> String {
    if value
        .rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case(extension))
    {
        value.to_string()
    } else {
        format!("{value}.{extension}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULE_NAME: &str = "sdk.rdb.patcher";

    #[test]
    fn requiring_rdb_patcher_adds_character_methods() {
        let lua = Lua::new();
        lua_api::install_runtime(&lua).expect("runtime");
        lua_api::authorize_character_extension_owner(&lua, MODULE_NAME).expect("authorize");
        let module = rdb_patcher_module(&lua).expect("module");
        lua_api::register_module(&lua, MODULE_NAME, module).expect("register");

        let ok: bool = lua
            .load(
                r#"
                local character = require("std.character")
                require("sdk.rdb.patcher")
                local law = character.find("law")
                law:replace_costume(3, "my_model.g1m")
                law:replace_portrait(2, "portrait.g1t")
                return law.replace_costume ~= nil
                    and law.replace_portrait ~= nil
                    and law.replace_model ~= nil
                    and law.replace_textures ~= nil
            "#,
            )
            .eval()
            .expect("eval");

        assert!(ok);
    }

    #[test]
    fn parses_texture_replacement_pairs() {
        let lua = Lua::new();
        let pairs: Vec<(String, String)> = lua
            .load(
                r#"
                return {
                    { "body", "my_super_body.g1t" },
                    { "left_arms", "my_super_left_arms.g1t" },
                }
            "#,
            )
            .eval::<Table>()
            .and_then(texture_pairs)
            .expect("pairs");

        assert_eq!(
            pairs,
            [
                ("body".to_string(), "my_super_body.g1t".to_string()),
                (
                    "left_arms".to_string(),
                    "my_super_left_arms.g1t".to_string()
                ),
            ]
        );
    }

    #[test]
    fn model_target_uses_character_model_stem() {
        let lua = Lua::new();
        let character = lua.create_table().expect("table");
        character.set("model_stem", "MPLC025_Garp").expect("stem");

        assert_eq!(
            model_target_file(&character, "default").expect("target"),
            "MPLC025_Garp.g1m"
        );
    }

    #[test]
    fn texture_target_uses_costume_body_part_asset_path() {
        let lua = Lua::new();
        let character = lua.create_table().expect("character");
        let costumes = lua.create_table().expect("costumes");
        let costume = lua.create_table().expect("costume");
        let body_parts = lua.create_table().expect("body parts");
        let body = lua.create_table().expect("body");
        let assets = lua.create_table().expect("assets");
        let asset = lua.create_table().expect("asset");
        asset.set("kind", "texture").expect("kind");
        asset.set("path", "MPLC025_Garp_body.g1t").expect("path");
        assets.set(1, asset).expect("asset set");
        body.set("assets", assets).expect("assets set");
        body_parts.set("body", body).expect("part set");
        costume.set("body_parts", body_parts).expect("parts set");
        costumes.set("default", costume).expect("costume set");
        character.set("costumes", costumes).expect("costumes set");

        assert_eq!(
            texture_target_file(&character, "default", "body").expect("target"),
            "MPLC025_Garp_body.g1t"
        );
    }
}
