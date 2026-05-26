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
        Err(error) => {
            log::write_global(format!("moveset_patcher module register error: {error}"));
            -2
        }
    }
}

fn moveset_patcher_module(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("id", PLUGIN_ID)?;
    table.set(
        "patch",
        lua.load(
            r#"
            return function(definition)
                local trace = rawget(_G, "__oppw4_trace")
                if trace ~= nil then trace("moveset_patcher.patch enter") end
                if trace ~= nil then
                    trace("moveset_patcher.patch keys payload_file=" .. tostring(definition.payload_file) .. " payload=" .. tostring(definition.payload ~= nil))
                end
                if trace ~= nil then trace("moveset_patcher.patch exit") end
                return definition
            end
            "#,
        )
        .eval::<Function>()?,
    )?;
    table.set(
        "__patch_from_source",
        lua.create_function(patch_from_source)?,
    )?;
    table.set(
        "load_patch",
        lua.load(
            r#"
            return function(module, path)
                local cache = rawget(_G, "__oppw4_mod_file_cache")
                if cache ~= nil then
                    local source = cache[path]
                    if source == nil then
                        source = cache[(path:gsub("\\", "/"))]
                    end
                    if source ~= nil then
                        return module.__patch_from_source(path, source)
                    end
                end
                return module.__load_patch_fallback(path)
            end
            "#,
        )
        .eval::<Function>()?
        .bind(table.clone())?,
    )?;
    table.set("__load_patch_fallback", lua.create_function(load_patch)?)?;
    register_character_extensions(lua)?;
    Ok(table)
}

fn register_character_extensions(lua: &Lua) -> mlua::Result<()> {
    log::write_global("register_character_extensions start");
    let method = replace_movesets_method(lua)?;
    if character_method_tables_exist(lua)? {
        lua_api::authorize_character_extension_owner(lua, PLUGIN_ID)?;
        register_character_method_direct(lua, PLUGIN_ID, "replace_movesets", method)?;
    } else {
        let register: Function = lua.globals().get("__oppw4_register_character_method")?;
        register.call::<()>((PLUGIN_ID, "replace_movesets", method))?;
    }
    log::write_global("register_character_extensions ok");
    Ok(())
}

fn replace_movesets_method(lua: &Lua) -> mlua::Result<Function> {
    let internal = lua.create_function(replace_movesets_direct)?;
    lua.load(
        r#"
        return function(internal)
            return function(character, moveset)
                local trace = rawget(_G, "__oppw4_trace")
                if trace ~= nil then trace("replace_movesets lua enter") end
                local entry = character.moveset_linkdata_entry
                if trace ~= nil then trace("replace_movesets lua entry=" .. tostring(entry)) end
                if entry == nil then
                    local name = character.canonical or character.name or "unknown"
                    error("no SDK moveset target for character=" .. tostring(name) .. "; add movesets.json in oppw4-data or pass a custom SDK character handle")
                end
                local name = character.canonical or character.name or "unknown"
                if trace ~= nil then
                    trace("replace_movesets lua call internal name=" .. tostring(name) .. " payload_file=" .. tostring(moveset.payload_file) .. " payload=" .. tostring(moveset.payload ~= nil))
                end
                return internal(entry, name, moveset.payload_file, moveset.payload)
            end
        end
        "#,
    )
    .eval::<Function>()?
    .call(internal)
}

fn character_method_tables_exist(lua: &Lua) -> mlua::Result<bool> {
    let globals = lua.globals();
    Ok(globals
        .get::<Option<Table>>("__struct_api_methods")?
        .is_some()
        && globals
            .get::<Option<Table>>("__struct_api_method_owners")?
            .is_some()
        && globals
            .get::<Option<Table>>("__struct_api_authorized_method_owners")?
            .is_some())
}

fn register_character_method_direct(
    lua: &Lua,
    owner: &str,
    name: &str,
    method: Function,
) -> mlua::Result<()> {
    let globals = lua.globals();
    let methods: Table = globals.get("__struct_api_methods")?;
    let owners: Table = globals.get("__struct_api_method_owners")?;
    let authorized: Table = globals.get("__struct_api_authorized_method_owners")?;
    let owner_key = owner.to_ascii_lowercase();
    if !authorized
        .get::<Option<bool>>(owner_key.as_str())?
        .unwrap_or(false)
    {
        return Err(mlua::Error::external(format!(
            "character.{name} refused for {owner}: missing std.character.extend"
        )));
    }
    if let Some(existing_owner) = owners.get::<Option<String>>(name)? {
        if !existing_owner.eq_ignore_ascii_case(&owner_key) {
            return Err(mlua::Error::external(format!(
                "character.{name} already registered by {existing_owner}, refused by {owner}"
            )));
        }
    }
    owners.set(name, owner_key)?;
    methods.set(name, method)
}

fn patch_definition(lua: &Lua, definition: Table) -> mlua::Result<Table> {
    log::write_global("patch_definition start");
    if let Some(path) = definition.get::<Option<String>>("payload_file")? {
        log::write_global(format!("patch_definition deferred payload_file={path}"));
        let output = lua.create_table()?;
        output.set("payload_file", path)?;
        if let Some(entry) = definition.get::<Option<u16>>("entry")? {
            output.set("source_entry", entry)?;
        }
        log::write_global("patch_definition ok deferred");
        return Ok(output);
    }

    let (payload, file_entry) = (payload::from_lua_table(definition.clone())?, None);

    let output = lua.create_table()?;
    if let Some(entry) = definition.get::<Option<u16>>("entry")?.or(file_entry) {
        output.set("source_entry", entry)?;
    }
    output.set("payload", lua.create_string(&payload)?)?;
    let entry = output.get::<Option<u16>>("source_entry")?.unwrap_or(0);
    log::write_global(format!(
        "patch_definition ok source_entry={entry} payload_bytes={}",
        payload.len()
    ));
    Ok(output)
}

fn load_patch(lua: &Lua, path: String) -> mlua::Result<Table> {
    log::write_global(format!("load_patch start path={path}"));
    let source = lua_api::read_mod_text(lua, Path::new(&path))?;
    log::write_global(format!(
        "load_patch read path={path} bytes={}",
        source.len()
    ));
    patch_from_source(lua, (path, source))
}

fn patch_from_source(lua: &Lua, (path, source): (String, String)) -> mlua::Result<Table> {
    log::write_global(format!(
        "load_patch source path={path} bytes={}",
        source.len()
    ));
    let definition = lua.load(&source).set_name(path).eval::<Table>()?;
    let patch = patch_definition(lua, definition)?;
    let entry = patch.get::<Option<u16>>("source_entry")?.unwrap_or(0);
    let payload_len = patch
        .get::<Option<mlua::String>>("payload")?
        .map(|payload| payload.as_bytes().len())
        .unwrap_or(0);
    log::write_global(format!(
        "load_patch ok source_entry={entry} payload_bytes={payload_len}"
    ));
    Ok(patch)
}

fn read_payload_file(lua: &Lua, path: &Path) -> mlua::Result<(Vec<u8>, Option<u16>)> {
    log::write_global(format!("read_payload_file start path={}", path.display()));
    let bytes = lua_api::read_mod_bytes(lua, path)?;
    log::write_global(format!(
        "read_payload_file bytes path={} len={}",
        path.display(),
        bytes.len()
    ));
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("json") => {
            let text = String::from_utf8(bytes).map_err(mlua::Error::external)?;
            log::write_global(format!(
                "read_payload_file json parse start path={} len={}",
                path.display(),
                text.len()
            ));
            let entry = payload::json_entry(&text).map_err(mlua::Error::external)?;
            let bytes = payload::from_json_str(&text).map_err(mlua::Error::external)?;
            log::write_global(format!(
                "read_payload_file json ok path={} payload_bytes={}",
                path.display(),
                bytes.len()
            ));
            Ok((bytes, entry))
        }
        Some(extension) if extension.eq_ignore_ascii_case("bin") => {
            log::write_global(format!(
                "read_payload_file bin ok path={} payload_bytes={}",
                path.display(),
                bytes.len()
            ));
            Ok((bytes, None))
        }
        Some(extension)
            if extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("hex") =>
        {
            let text = String::from_utf8(bytes).map_err(mlua::Error::external)?;
            log::write_global(format!(
                "read_payload_file hex parse start path={} len={}",
                path.display(),
                text.len()
            ));
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

fn replace_movesets_direct(
    lua: &Lua,
    (entry, character_name, payload_file, payload): (
        u16,
        String,
        Option<String>,
        Option<mlua::String>,
    ),
) -> mlua::Result<()> {
    let payload = if let Some(payload) = payload {
        payload.as_bytes().to_vec()
    } else if let Some(path) = payload_file {
        log::write_global(format!("replace_movesets payload_file start path={path}"));
        let (payload, file_entry) = read_payload_file(lua, Path::new(&path))?;
        if let Some(file_entry) = file_entry {
            log::write_global(format!(
                "replace_movesets payload_file entry_hint={file_entry} ignored target_entry={entry}"
            ));
        }
        payload
    } else {
        return Err(mlua::Error::external(
            "replace_movesets expects moveset.payload or moveset.payload_file",
        ));
    };
    let payload_len = payload.len();
    log::write_global(format!(
        "replace_movesets start character={character_name} entry={entry} bytes={payload_len}"
    ));
    state::replace_entry(entry as usize, &payload).map_err(mlua::Error::external)?;
    log::write_global(format!(
        "moveset patch registered character={character_name} entry={entry} bytes={payload_len} patches={}",
        state::edit_count()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn requiring_moveset_patcher_adds_replace_method() {
        let lua = Lua::new();
        lua_api::install_runtime(&lua).expect("runtime");
        install_test_character_module(&lua);
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

    fn install_test_character_module(lua: &Lua) {
        let authorized = lua.create_table().expect("authorized owners");
        lua.globals()
            .set("__struct_api_authorized_method_owners", authorized.clone())
            .expect("authorized global");

        let garp = lua.create_table().expect("garp");
        garp.set("canonical", "garp").expect("canonical");
        let extension_target = garp.clone();
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
                    if name == "garp" {
                        Ok(Some(garp.clone()))
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

        replace_movesets_direct(
            &lua,
            (
                character
                    .get::<u16>("moveset_linkdata_entry")
                    .expect("target entry"),
                character.get::<String>("canonical").expect("canonical"),
                None,
                Some(moveset.get::<mlua::String>("payload").expect("payload")),
            ),
        )
        .expect("replace");

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

        let method = replace_movesets_method(&lua).expect("method");
        let error = method
            .call::<()>((character, moveset))
            .expect_err("missing target");

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
