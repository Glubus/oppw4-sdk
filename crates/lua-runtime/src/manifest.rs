use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaModManifest {
    pub id: String,
    pub name: String,
    pub uses_plugins: Vec<String>,
    pub entry_lua: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaMod {
    pub manifest: LuaModManifest,
    pub source: ModSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModSource {
    Directory(PathBuf),
    Zip { path: PathBuf, root: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModManifestError {
    InvalidToml,
    MissingModTable,
    MissingId,
    MissingEntry,
    InvalidEntryPath,
}

pub fn discover_mods(root: &Path) -> Vec<LuaMod> {
    let mut mods = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return mods;
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            collect_directory_mod(&path, &mut mods);
        } else if file_type.is_file() && is_zip_file(&path) {
            collect_zip_mod(&path, &mut mods);
        }
    }
    mods.sort_by_key(|entry| entry.manifest.id.to_ascii_lowercase());
    mods
}

pub fn parse_mod_manifest(text: &str) -> Result<LuaModManifest, ModManifestError> {
    let value = text
        .parse::<toml::Value>()
        .map_err(|_| ModManifestError::InvalidToml)?;
    let mod_table = value
        .get("mod")
        .and_then(toml::Value::as_table)
        .ok_or(ModManifestError::MissingModTable)?;
    let id = mod_table
        .get("id")
        .and_then(toml::Value::as_str)
        .map(sanitize_id)
        .filter(|id| !id.is_empty())
        .ok_or(ModManifestError::MissingId)?;
    let name = mod_table
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let uses_plugins = value
        .get("uses")
        .and_then(|uses| uses.get("plugins"))
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(sanitize_id)
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let entry_lua = value
        .get("entry")
        .and_then(|entry| entry.get("lua"))
        .and_then(toml::Value::as_str)
        .ok_or(ModManifestError::MissingEntry)?;
    if !is_safe_relative_file(entry_lua) {
        return Err(ModManifestError::InvalidEntryPath);
    }

    Ok(LuaModManifest {
        id,
        name,
        uses_plugins,
        entry_lua: entry_lua.replace('\\', "/"),
    })
}

impl LuaMod {
    pub fn uses_plugin(&self, plugin_id: &str) -> bool {
        self.manifest
            .uses_plugins
            .iter()
            .any(|id| id.eq_ignore_ascii_case(plugin_id))
    }

    pub fn is_zip(&self) -> bool {
        matches!(self.source, ModSource::Zip { .. })
    }

    pub fn source_path(&self) -> &Path {
        match &self.source {
            ModSource::Directory(path) | ModSource::Zip { path, .. } => path,
        }
    }

    pub fn read_entry_script(&self) -> std::io::Result<String> {
        match &self.source {
            ModSource::Directory(root) => {
                fs::read_to_string(root.join(self.manifest.entry_lua.replace('/', "\\")))
            }
            ModSource::Zip { path, root } => {
                read_zip_text(path, &zip_entry_path(root, &self.manifest.entry_lua))
            }
        }
    }
}

fn collect_directory_mod(path: &Path, mods: &mut Vec<LuaMod>) {
    let manifest_path = path.join("mod.toml");
    let Ok(text) = fs::read_to_string(manifest_path) else {
        return;
    };
    let Ok(manifest) = parse_mod_manifest(&text) else {
        return;
    };
    mods.push(LuaMod {
        manifest,
        source: ModSource::Directory(path.to_path_buf()),
    });
}

fn collect_zip_mod(path: &Path, mods: &mut Vec<LuaMod>) {
    let Ok((manifest_entry, text)) = read_zip_manifest(path) else {
        return;
    };
    let Ok(manifest) = parse_mod_manifest(&text) else {
        return;
    };
    let root = manifest_entry
        .rsplit_once('/')
        .map(|(root, _)| format!("{root}/"))
        .unwrap_or_default();
    mods.push(LuaMod {
        manifest,
        source: ModSource::Zip {
            path: path.to_path_buf(),
            root,
        },
    });
}

fn read_zip_manifest(path: &Path) -> std::io::Result<(String, String)> {
    if let Ok(text) = read_zip_text(path, "mod.toml") {
        return Ok(("mod.toml".to_string(), text));
    }

    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut matches = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        if is_nested_mod_manifest(&name) {
            matches.push(name);
        }
    }
    if matches.len() != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "zip must contain exactly one mod.toml",
        ));
    }

    let name = matches.remove(0);
    let text = read_zip_text(path, &name)?;
    Ok((name, text))
}

fn read_zip_text(path: &Path, entry_name: &str) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name(entry_name)?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(text)
}

fn zip_entry_path(root: &str, entry_name: &str) -> String {
    if root.is_empty() {
        entry_name.to_string()
    } else {
        format!("{root}{entry_name}")
    }
}

fn is_nested_mod_manifest(name: &str) -> bool {
    name.ends_with("/mod.toml") && !name.contains("../")
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn is_safe_relative_file(path: &str) -> bool {
    let path = Path::new(path);
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
    use std::io::Write;

    #[test]
    fn parses_lua_mod_manifest() {
        let manifest = parse_mod_manifest(
            r#"
                [mod]
                id = "zoro_fx"
                name = "Zoro FX"

                [uses]
                plugins = ["sdk_runtime"]

                [entry]
                lua = "mod.lua"
            "#,
        )
        .expect("manifest");

        assert_eq!(manifest.id, "zoro_fx");
        assert_eq!(manifest.name, "Zoro FX");
        assert_eq!(manifest.uses_plugins, ["sdk_runtime"]);
        assert_eq!(manifest.entry_lua, "mod.lua");
    }

    #[test]
    fn rejects_parent_entry_path() {
        let error = parse_mod_manifest(
            r#"
                [mod]
                id = "bad"

                [entry]
                lua = "../bad.lua"
            "#,
        )
        .expect_err("bad path");

        assert_eq!(error, ModManifestError::InvalidEntryPath);
    }

    #[test]
    fn discovers_zip_mod_with_nested_single_root_folder() {
        let root = std::env::temp_dir().join(format!("oppw4-lua-mod-zip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        let zip_path = root.join("aura_zoro_lua.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "aura_zoro_lua/mod.toml",
                    r#"
                        [mod]
                        id = "aura_zoro_lua"

                        [uses]
                        plugins = ["sdk_runtime"]

                        [entry]
                        lua = "mod.lua"
                    "#,
                ),
                ("aura_zoro_lua/mod.lua", r#"require("sdk.runtime.fx")"#),
            ],
        );

        let mods = discover_mods(&root);

        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].manifest.id, "aura_zoro_lua");
        assert!(mods[0].is_zip());
        assert_eq!(
            mods[0].read_entry_script().expect("script"),
            r#"require("sdk.runtime.fx")"#
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(path).expect("zip file");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, text) in entries {
            writer.start_file(name, options).expect("entry");
            writer.write_all(text.as_bytes()).expect("write");
        }
        writer.finish().expect("finish");
    }
}
