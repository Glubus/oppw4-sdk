use std::path::Path;

use crate::{ModManifest, ModManifestError};

pub fn parse_mod_manifest(text: &str) -> Result<ModManifest, ModManifestError> {
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
        .ok_or(ModManifestError::MissingEntry)?;
    if !is_safe_relative_file(entry_file) {
        return Err(ModManifestError::InvalidEntryPath);
    }

    Ok(ModManifest {
        id,
        name,
        runtime,
        uses_plugins,
        entry_file: entry_file.replace('\\', "/"),
    })
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

        assert_eq!(manifest.id, "zoro_fx");
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

        assert_eq!(error, ModManifestError::MissingEntry);
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

        assert_eq!(error, ModManifestError::InvalidEntryPath);
    }
}
