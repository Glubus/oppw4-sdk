use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use mlua::Lua;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModFileSource {
    File(PathBuf),
    ZipEntry {
        zip_path: PathBuf,
        entry_name: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CurrentModFileContext {
    pub root: PathBuf,
    pub zip_root: String,
    pub is_zip: bool,
    pub files: HashMap<String, Vec<u8>>,
}

thread_local! {
    static CURRENT_MOD_FILE_CONTEXT: RefCell<Option<CurrentModFileContext>> = const { RefCell::new(None) };
}

pub fn set_current_mod_file_context(context: Option<CurrentModFileContext>) {
    CURRENT_MOD_FILE_CONTEXT.with(|current| {
        *current.borrow_mut() = context;
    });
}

pub fn read_mod_text(lua: &Lua, path: impl AsRef<Path>) -> mlua::Result<String> {
    let path = path.as_ref();
    if current_mod_is_zip(lua)? {
        let bytes = read_mod_bytes(lua, path)?;
        return String::from_utf8(bytes).map_err(|error| {
            mlua::Error::external(format!(
                "mod file {} is not valid UTF-8: {error}",
                path.display()
            ))
        });
    }
    let path = resolve_directory_mod_file(lua, path)?;
    fs::read_to_string(&path).map_err(|error| {
        mlua::Error::external(format!(
            "failed to read mod text file {}: {error}",
            path.display()
        ))
    })
}

pub fn read_current_mod_text(path: impl AsRef<Path>) -> Option<Result<String, String>> {
    let path = path.as_ref();
    read_current_mod_bytes(path).map(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("mod file {} is not valid UTF-8: {error}", path.display()))
    })
}

pub fn read_cached_mod_text(lua: &Lua, path: impl AsRef<Path>) -> mlua::Result<Option<String>> {
    let Some(key) = cache_key(path.as_ref()) else {
        return Ok(None);
    };
    let Some(cache) = lua
        .globals()
        .get::<Option<mlua::Table>>("__oppw4_mod_file_cache")?
    else {
        return Ok(None);
    };
    cache.get::<Option<String>>(key)
}

pub fn read_current_mod_bytes(path: impl AsRef<Path>) -> Option<Vec<u8>> {
    cached_mod_file(path.as_ref())
}

pub fn read_mod_bytes(lua: &Lua, path: impl AsRef<Path>) -> mlua::Result<Vec<u8>> {
    let path = path.as_ref();
    if let Some(bytes) = cached_mod_file(path) {
        return Ok(bytes);
    }
    match resolve_mod_file_source(lua, path)? {
        ModFileSource::File(path) => fs::read(&path).map_err(|error| {
            mlua::Error::external(format!(
                "failed to read mod file {}: {error}",
                path.display()
            ))
        }),
        ModFileSource::ZipEntry {
            zip_path,
            entry_name,
        } => read_zip_entry(&zip_path, &entry_name).map_err(|error| {
            mlua::Error::external(format!(
                "failed to read mod zip entry {entry_name} from {}: {error}",
                zip_path.display()
            ))
        }),
    }
}

pub fn resolve_mod_file_source(lua: &Lua, path: impl AsRef<Path>) -> mlua::Result<ModFileSource> {
    let path = path.as_ref();
    if !is_safe_relative_file(path) {
        return Err(mlua::Error::external(format!(
            "mod file path must be relative to the current mod folder: {}",
            path.display()
        )));
    }
    if current_mod_is_zip(lua)? {
        let zip_path = current_mod_root(lua)?;
        let zip_root = current_mod_zip_root(lua)?;
        return Ok(ModFileSource::ZipEntry {
            zip_path,
            entry_name: zip_entry_path(&zip_root, path),
        });
    }
    Ok(ModFileSource::File(resolve_directory_mod_file(lua, path)?))
}

fn cached_mod_file(path: &Path) -> Option<Vec<u8>> {
    let key = cache_key(path)?;
    CURRENT_MOD_FILE_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .and_then(|context| context.files.get(&key).cloned())
    })
}

pub fn cache_key(path: &Path) -> Option<String> {
    if !is_safe_relative_file(path) {
        return None;
    }
    Some(path.to_string_lossy().replace('\\', "/"))
}

fn resolve_directory_mod_file(lua: &Lua, path: &Path) -> mlua::Result<PathBuf> {
    if !is_safe_relative_file(path) {
        return Err(mlua::Error::external(format!(
            "mod file path must be relative to the current mod folder: {}",
            path.display()
        )));
    }
    Ok(current_mod_root(lua)?.join(path))
}

fn current_mod_root(lua: &Lua) -> mlua::Result<PathBuf> {
    if let Some(root) = CURRENT_MOD_FILE_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|context| context.root.clone())
    }) {
        return Ok(root);
    }
    let root = lua
        .globals()
        .get::<Option<String>>("__oppw4_mod_root")?
        .ok_or_else(|| mlua::Error::external("missing current Lua mod root"))?;
    Ok(PathBuf::from(root))
}

fn current_mod_zip_root(lua: &Lua) -> mlua::Result<String> {
    if let Some(zip_root) = CURRENT_MOD_FILE_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|context| context.zip_root.clone())
    }) {
        return Ok(zip_root);
    }
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_zip_root")
        .map(|root| root.unwrap_or_default())
}

fn current_mod_is_zip(lua: &Lua) -> mlua::Result<bool> {
    if let Some(is_zip) = CURRENT_MOD_FILE_CONTEXT
        .with(|current| current.borrow().as_ref().map(|context| context.is_zip))
    {
        return Ok(is_zip);
    }
    Ok(lua
        .globals()
        .get::<Option<bool>>("__oppw4_mod_is_zip")?
        .unwrap_or(false))
}

fn read_zip_entry(path: &Path, entry_name: &str) -> std::io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_error)?;
    let mut entry = archive.by_name(entry_name).map_err(zip_error)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn zip_entry_path(root: &str, entry_name: &Path) -> String {
    let entry_name = entry_name.to_string_lossy().replace('\\', "/");
    if root.is_empty() {
        entry_name
    } else {
        format!("{root}{entry_name}")
    }
}

fn zip_error(error: zip::result::ZipError) -> std::io::Error {
    std::io::Error::other(error)
}

fn is_safe_relative_file(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
        && path.file_name().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_text_from_current_directory_mod() {
        let lua = Lua::new();
        let root = std::env::temp_dir().join(format!("oppw4-lua-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(root.join("data.txt"), "ok").expect("write");
        lua.globals()
            .set("__oppw4_mod_root", root.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let text = read_mod_text(&lua, "data.txt").expect("text");

        assert_eq!(text, "ok");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_text_from_nested_zip_mod() {
        let lua = Lua::new();
        let root = std::env::temp_dir().join(format!("oppw4-lua-file-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let zip_path = root.join("moveset.zip");
        write_zip(&zip_path, &[("nested/data.txt", "ok")]);
        lua.globals()
            .set("__oppw4_mod_root", zip_path.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_zip_root", "nested/")
            .expect("zip root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", true)
            .expect("zip global");

        let text = read_mod_text(&lua, "data.txt").expect("text");

        assert_eq!(text, "ok");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_zip_entry_reports_path_and_archive() {
        let lua = Lua::new();
        let root =
            std::env::temp_dir().join(format!("oppw4-lua-file-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let zip_path = root.join("moveset.zip");
        write_zip(&zip_path, &[("nested/other.txt", "ok")]);
        lua.globals()
            .set("__oppw4_mod_root", zip_path.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_zip_root", "nested/")
            .expect("zip root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", true)
            .expect("zip global");

        let error = read_mod_text(&lua, "data.txt").expect_err("missing file");
        let message = error.to_string();

        assert!(message.contains("nested/data.txt"));
        assert!(message.contains("moveset.zip"));
        let _ = std::fs::remove_dir_all(&root);
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
