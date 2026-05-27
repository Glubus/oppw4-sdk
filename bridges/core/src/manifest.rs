use std::{fs, io::Read, path::Path};

use crate::{BridgeLoadRequest, BridgeModSource, ModId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeModManifest {
    pub id: ModId,
    pub name: String,
    pub runtime: Option<String>,
    pub uses_plugins: Vec<String>,
    pub entry_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredBridgeMod {
    pub manifest: BridgeModManifest,
    pub source: BridgeModSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeModManifestError {
    InvalidToml,
    MissingModTable,
    MissingId,
    MissingEntry,
    InvalidEntryPath,
    InvalidModId,
}

impl DiscoveredBridgeMod {
    pub fn into_load_request(self) -> BridgeLoadRequest {
        BridgeLoadRequest {
            mod_id: self.manifest.id,
            name: self.manifest.name,
            source: self.source,
            entry_file: self.manifest.entry_file,
            uses_plugins: self.manifest.uses_plugins,
        }
    }
}

pub fn discover_mods(root: &Path) -> Vec<DiscoveredBridgeMod> {
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
    mods.sort_by_key(|entry| entry.manifest.id.as_str().to_ascii_lowercase());
    mods
}

pub fn parse_mod_manifest(text: &str) -> Result<BridgeModManifest, BridgeModManifestError> {
    let value = text
        .parse::<toml::Value>()
        .map_err(|_| BridgeModManifestError::InvalidToml)?;
    let mod_table = value
        .get("mod")
        .and_then(toml::Value::as_table)
        .ok_or(BridgeModManifestError::MissingModTable)?;
    let id = mod_table
        .get("id")
        .and_then(toml::Value::as_str)
        .map(sanitize_id)
        .filter(|id| !id.is_empty())
        .ok_or(BridgeModManifestError::MissingId)?;
    let id = ModId::new(id).map_err(|_| BridgeModManifestError::InvalidModId)?;
    let name = mod_table
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| id.as_str())
        .to_string();
    let runtime = mod_table
        .get("runtime")
        .and_then(toml::Value::as_str)
        .map(sanitize_id)
        .filter(|runtime| !runtime.is_empty());
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
    let entry_file = value
        .get("entry")
        .and_then(|entry| entry.get("file").or_else(|| entry.get("main")))
        .and_then(toml::Value::as_str)
        .ok_or(BridgeModManifestError::MissingEntry)?;
    if !is_safe_relative_file(entry_file) {
        return Err(BridgeModManifestError::InvalidEntryPath);
    }

    Ok(BridgeModManifest {
        id,
        name,
        runtime,
        uses_plugins,
        entry_file: entry_file.replace('\\', "/"),
    })
}

fn collect_directory_mod(path: &Path, mods: &mut Vec<DiscoveredBridgeMod>) {
    let manifest_path = path.join("mod.toml");
    let Ok(text) = fs::read_to_string(manifest_path) else {
        return;
    };
    let Ok(manifest) = parse_mod_manifest(&text) else {
        return;
    };
    mods.push(DiscoveredBridgeMod {
        manifest,
        source: BridgeModSource::Directory(path.to_path_buf()),
    });
}

fn collect_zip_mod(path: &Path, mods: &mut Vec<DiscoveredBridgeMod>) {
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
    mods.push(DiscoveredBridgeMod {
        manifest,
        source: BridgeModSource::Zip {
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

    #[test]
    fn parses_mod_manifest() {
        let manifest = parse_mod_manifest(
            r#"
                [mod]
                id = "zoro_fx"
                name = "Zoro FX"

                [uses]
                plugins = ["sdk_runtime"]

                [entry]
                file = "main.mod"
            "#,
        )
        .expect("manifest");

        assert_eq!(manifest.id.as_str(), "zoro_fx");
        assert_eq!(manifest.name, "Zoro FX");
        assert_eq!(manifest.runtime, None);
        assert_eq!(manifest.uses_plugins, ["sdk_runtime"]);
        assert_eq!(manifest.entry_file, "main.mod");
    }

    #[test]
    fn parses_optional_runtime_hint() {
        let manifest = parse_mod_manifest(
            r#"
                [mod]
                id = "native_mod"
                runtime = "rust-native"

                [entry]
                file = "native_mod.dll"
            "#,
        )
        .expect("manifest");

        assert_eq!(manifest.runtime.as_deref(), Some("rust-native"));
    }

    #[test]
    fn rejects_specific_entry_key_manifest() {
        let error = parse_mod_manifest(
            r#"
                [mod]
                id = "legacy_entry"

                [entry]
                script = "main.mod"
            "#,
        )
        .expect_err("missing generic entry");

        assert_eq!(error, BridgeModManifestError::MissingEntry);
    }

    #[test]
    fn rejects_parent_entry_path() {
        let error = parse_mod_manifest(
            r#"
                [mod]
                id = "bad"

                [entry]
                file = "../bad.mod"
            "#,
        )
        .expect_err("bad path");

        assert_eq!(error, BridgeModManifestError::InvalidEntryPath);
    }
}
