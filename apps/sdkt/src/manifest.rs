use std::{fs, path::Path};

use crate::{report::Diagnostic, sources::is_script_file};
use sdk_mod_loader::parse_mod_manifest;

pub(crate) fn manifest_diagnostics(roots: &[std::path::PathBuf]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for root in roots {
        if root.is_dir() {
            let manifest = root.join("mod.toml");
            if manifest.exists() {
                validate_manifest(root, &manifest, &mut diagnostics);
            }
        } else if root.file_name().is_some_and(|name| name == "mod.toml") {
            let root = root.parent().unwrap_or_else(|| Path::new("."));
            validate_manifest(root, root.join("mod.toml").as_path(), &mut diagnostics);
        }
    }
    diagnostics
}

fn validate_manifest(root: &Path, manifest: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let text = match fs::read_to_string(manifest) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                manifest,
                "manifest_read_failed",
                format!("failed to read mod.toml: {error}"),
            ));
            return;
        }
    };
    let manifest_data = match parse_mod_manifest(&text) {
        Ok(manifest_data) => manifest_data,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                manifest,
                "manifest_invalid",
                format!("invalid mod.toml: {error:?}"),
            ));
            return;
        }
    };
    let entry = root.join(&manifest_data.entry_file);
    if !entry.exists() {
        diagnostics.push(Diagnostic::error(
            manifest,
            "entry_missing",
            format!("entry file does not exist: {}", manifest_data.entry_file),
        ));
    } else if !is_script_file(&entry) {
        diagnostics.push(Diagnostic::warning(
            manifest,
            "entry_not_script",
            format!(
                "bridge-js accepts .js, .ts, .mjs and .mts entries: {}",
                manifest_data.entry_file
            ),
        ));
    }
    if manifest_data.uses_plugins.is_empty() {
        diagnostics.push(Diagnostic::warning(
            manifest,
            "uses_plugins_empty",
            "mod.toml does not declare [uses].plugins",
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::{env, time::SystemTime};

    use super::*;

    #[test]
    fn validates_mod_manifest_entry() {
        let root = unique_temp_dir("manifest-entry");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("mod.toml"),
            r#"
            [mod]
            id = "example"

            [uses]
            plugins = ["sdk_runtime"]

            [entry]
            file = "main.ts"
            "#,
        )
        .expect("manifest");
        fs::write(root.join("main.ts"), "").expect("entry");

        let diagnostics = manifest_diagnostics(&[root.clone()]);

        assert!(diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_manifest_entry() {
        let root = unique_temp_dir("missing-entry");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("mod.toml"),
            r#"
            [mod]
            id = "example"

            [entry]
            file = "missing.ts"
            "#,
        )
        .expect("manifest");

        let diagnostics = manifest_diagnostics(&[root.clone()]);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "entry_missing"));
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
