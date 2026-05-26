use std::{ffi::c_void, path::Path};

use mlua::{Function, Lua, Table};
use plugin_sdk::{HostApi, PluginError};

use crate::{constants::PLUGIN_ID, log, payload, state};

pub(crate) fn register(host: HostApi<'_>) {
    let result = match host.lua().register_module_fn(
        PLUGIN_ID,
        PLUGIN_ID,
        std::ptr::null_mut(),
        register_moveset_patcher_module,
    ) {
        Ok(()) => 0,
        Err(PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };
    if result != 0 {
        log::write(
            host,
            format!("moveset_patcher lua module register failed result={result}"),
        );
    }
}

unsafe extern "system" fn register_moveset_patcher_module(
    _context: *mut c_void,
    lua: *mut c_void,
) -> i32 {
    let Some(lua) = lua.cast::<Lua>().as_ref() else {
        return -1;
    };
    match moveset_patcher_module(lua)
        .and_then(|table| lua_api::register_module(lua, PLUGIN_ID, table))
    {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

fn moveset_patcher_module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", PLUGIN_ID)?;
    table.set("patch", lua.create_function(patch_definition)?)?;
    table.set("load_patch", lua.create_function(load_patch)?)?;
    table.set(
        "__oppw4_on_import",
        lua.create_function(|lua, ()| register_character_extensions(lua))?,
    )?;
    Ok(table)
}

fn register_character_extensions(lua: &Lua) -> mlua::Result<()> {
    let register: Function = lua.globals().get("__oppw4_register_character_method")?;
    register.call::<()>((
        PLUGIN_ID,
        "replace_movesets",
        lua.create_function(replace_movesets)?,
    ))
}

fn patch_definition(lua: &Lua, definition: Table) -> mlua::Result<Table> {
    let (payload, file_entry) =
        if let Some(path) = definition.get::<Option<String>>("payload_file")? {
            read_payload_file(lua, Path::new(&path))?
        } else {
            (payload::from_lua_table(definition.clone())?, None)
        };

    let output = lua.create_table()?;
    if let Some(entry) = definition.get::<Option<u16>>("entry")?.or(file_entry) {
        output.set("source_entry", entry)?;
    }
    output.set("payload", lua.create_string(&payload)?)?;
    Ok(output)
}

fn load_patch(lua: &Lua, path: String) -> mlua::Result<Table> {
    let source = lua_api::read_mod_text(lua, Path::new(&path))?;
    let definition = lua.load(&source).set_name(path).eval::<Table>()?;
    patch_definition(lua, definition)
}

fn read_payload_file(lua: &Lua, path: &Path) -> mlua::Result<(Vec<u8>, Option<u16>)> {
    let bytes = lua_api::read_mod_bytes(lua, path)?;
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("json") => {
            let text = String::from_utf8(bytes).map_err(mlua::Error::external)?;
            let entry = payload::json_entry(&text).map_err(mlua::Error::external)?;
            let bytes = payload::from_json_str(&text).map_err(mlua::Error::external)?;
            Ok((bytes, entry))
        }
        Some(extension) if extension.eq_ignore_ascii_case("bin") => Ok((bytes, None)),
        Some(extension)
            if extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("hex") =>
        {
            let text = String::from_utf8(bytes).map_err(mlua::Error::external)?;
            crate::hex::parse_payload(&text)
                .map(|payload| (payload, None))
                .map_err(mlua::Error::external)
        }
        Some(extension) => Err(mlua::Error::external(format!(
            "unsupported patch extension: {extension}"
        ))),
        None => Err(mlua::Error::external("patch missing extension")),
    }
}

fn replace_movesets(_: &Lua, (character, moveset): (Table, Table)) -> mlua::Result<()> {
    let entry = character.get::<Option<u16>>("moveset_linkdata_entry")?.ok_or_else(|| {
        let name = character_name(&character)
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());
        mlua::Error::external(format!(
            "no SDK moveset target for character={name}; add movesets.json in oppw4-data or pass a custom SDK character handle"
        ))
    })?;
    let payload = moveset
        .get::<Option<mlua::String>>("payload")?
        .ok_or_else(|| mlua::Error::external("replace_movesets expects moveset.payload"))?;
    let character_name = character_name(&character)?.unwrap_or_else(|| "unknown".to_string());
    let payload_len = payload.as_bytes().len();
    state::replace_entry(entry as usize, payload.as_bytes().as_ref())
        .map_err(mlua::Error::external)?;
    log::write_global(format!(
        "moveset patch registered character={character_name} entry={entry} bytes={payload_len} patches={}",
        state::edit_count()
    ));
    Ok(())
}

fn character_name(character: &Table) -> mlua::Result<Option<String>> {
    Ok(character
        .get::<Option<String>>("canonical")?
        .or_else(|| character.get::<Option<String>>("name").ok().flatten()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn requiring_moveset_patcher_adds_replace_method() {
        let lua = Lua::new();
        lua_api::install_runtime(&lua).expect("runtime");
        lua_api::authorize_character_extension_owner(&lua, PLUGIN_ID).expect("authorize");
        let module = moveset_patcher_module(&lua).expect("module");
        lua_api::register_module(&lua, PLUGIN_ID, module).expect("register");

        let has_method: bool = lua
            .load(
                r#"
                local character = require("std.character")
                require("moveset_patcher")
                local garp = character.find("garp")
                return garp.replace_movesets ~= nil
            "#,
            )
            .eval()
            .expect("eval");

        assert!(has_method);
    }

    #[test]
    fn readable_section_payload_builds_definition() {
        let lua = Lua::new();
        let definition = lua
            .load(
                r#"
                return {
                    entry = 247,
                    section_count = 2,
                    sections = {
                        { index = 0, record_size = 16, records = {
                            { 1, 2, 3, 4 },
                        }},
                        { index = 1, words = { 5, 6 } },
                    },
                }
            "#,
            )
            .eval::<Table>()
            .expect("table");

        let moveset = patch_definition(&lua, definition).expect("definition");

        assert_eq!(
            moveset.get::<u16>("source_entry").expect("source_entry"),
            247
        );
        let payload = moveset.get::<mlua::String>("payload").expect("payload");
        assert!(!payload.as_bytes().is_empty());
    }

    #[test]
    fn load_patch_loads_readable_lua_table() {
        let lua = Lua::new();
        let root =
            std::env::temp_dir().join(format!("oppw4-moveset-readable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(
            root.join("moveset.lua"),
            r#"
            return {
              entry = 247,
              section_count = 1,
              sections = {
                { index = 0, record_size = 16, records = {
                  { 1, 2, 3, 4 },
                }},
              },
            }
            "#,
        )
        .expect("write");
        lua.globals()
            .set("__oppw4_mod_root", root.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let moveset = load_patch(&lua, "moveset.lua".to_string()).expect("moveset file");

        assert_eq!(
            moveset.get::<u16>("source_entry").expect("source_entry"),
            247
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_patch_loads_nested_zip_payload() {
        let lua = Lua::new();
        let root =
            std::env::temp_dir().join(format!("oppw4-moveset-zip-readable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let zip_path = root.join("okiku_moveset.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "okiku_moveset/mod.lua",
                    r#"local moveset_patcher = require("moveset_patcher")"#,
                ),
                (
                    "okiku_moveset/moveset.lua",
                    r#"
                    return {
                      entry = 231,
                      section_count = 1,
                      sections = {
                        { index = 0, record_size = 16, records = {
                          { 1, 2, 3, 4 },
                        }},
                      },
                    }
                    "#,
                ),
            ],
        );
        lua.globals()
            .set("__oppw4_mod_root", zip_path.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_zip_root", "okiku_moveset/")
            .expect("zip root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", true)
            .expect("zip global");

        let moveset = load_patch(&lua, "moveset.lua".to_string()).expect("moveset file");

        assert_eq!(
            moveset.get::<u16>("source_entry").expect("source_entry"),
            231
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_patch_loads_dumped_entry_when_available() {
        let path = std::env::var_os("OPPW4_MOVESET_DUMP_TEST")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(
                    r"D:\SteamLibrary\steamapps\common\OPPW4\mods\moveset_probe\entry_0069.lua",
                )
            });
        if !path.is_file() {
            return;
        }

        let lua = Lua::new();
        let root = path.parent().expect("dump parent");
        lua.globals()
            .set("__oppw4_mod_root", root.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let moveset = load_patch(
            &lua,
            path.file_name()
                .expect("dump filename")
                .to_string_lossy()
                .to_string(),
        )
        .expect("dumped moveset file");

        assert_eq!(
            moveset.get::<u16>("source_entry").expect("source_entry"),
            69
        );
        let payload = moveset.get::<mlua::String>("payload").expect("payload");
        assert!(!payload.as_bytes().is_empty());
    }

    #[test]
    fn replace_movesets_uses_sdk_character_target_entry() {
        crate::state::initialize(plugin_sdk::HostApi::from(&test_linkdata_api())).ok();

        let lua = Lua::new();
        let character = lua
            .load(
                r#"
                return {
                    canonical = "garp",
                    moveset_linkdata_entry = 247,
                }
                "#,
            )
            .eval::<Table>()
            .expect("character");
        let moveset = lua.create_table().expect("moveset");
        moveset.set("source_entry", 69u16).expect("source entry");
        moveset
            .set(
                "payload",
                lua.create_string([1u8, 2, 3, 4]).expect("payload"),
            )
            .expect("payload");

        replace_movesets(&lua, (character, moveset)).expect("replace");

        let edits = crate::state::edit_count();
        assert!(edits >= 1);
    }

    #[test]
    fn replace_movesets_rejects_character_without_sdk_target_entry() {
        let lua = Lua::new();
        let character = lua
            .load(
                r#"
                return {
                    canonical = "garp",
                }
                "#,
            )
            .eval::<Table>()
            .expect("character");
        let moveset = lua.create_table().expect("moveset");
        moveset
            .set(
                "payload",
                lua.create_string([1u8, 2, 3, 4]).expect("payload"),
            )
            .expect("payload");

        let error = replace_movesets(&lua, (character, moveset)).expect_err("missing target");

        assert!(error.to_string().contains("no SDK moveset target"));
    }

    fn test_linkdata_api() -> plugin_sdk::Oppw4PluginApi {
        unsafe extern "system" fn replace_linkdata_entry(
            _host_context: *mut std::ffi::c_void,
            _patch: *const plugin_sdk::Oppw4LinkDataEntryPatch,
        ) -> i32 {
            0
        }

        plugin_sdk::Oppw4PluginApi {
            replace_linkdata_entry: Some(replace_linkdata_entry),
            ..plugin_abi::null_api()
        }
    }

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).expect("zip file");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, text) in entries {
            writer.start_file(*name, options).expect("zip entry");
            std::io::Write::write_all(&mut writer, text.as_bytes()).expect("zip write");
        }
        writer.finish().expect("finish zip");
    }
}
