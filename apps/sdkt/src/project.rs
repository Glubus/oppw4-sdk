use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::config::{self, SdktConfig};

pub(crate) fn normalize_root(path: &Path) -> PathBuf {
    if path
        .file_name()
        .is_some_and(|name| name == std::ffi::OsStr::new("mod.toml"))
    {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

pub(crate) fn init_project(root: &Path, force: bool) -> Result<(), String> {
    let root = normalize_root(root);
    fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;

    let config = SdktConfig {
        game_path: None,
        mods_path: Some(PathBuf::from(".sdkt/mods")),
        profile: Some("default".to_string()),
        default_bridge: Some(config::default_bridge()),
    };
    if force || !config::config_path(&root).exists() {
        config::save(&root, &config)?;
    }

    let source_dir = root.join("src");
    if force || !source_dir.exists() {
        fs::create_dir_all(&source_dir)
            .map_err(|error| format!("{}: {error}", source_dir.display()))?;
    }

    let manifest_path = root.join("mod.toml");
    if force || !manifest_path.exists() {
        fs::write(&manifest_path, manifest_template(&root))
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    }

    let entry_path = source_dir.join("main.ts");
    if force || !entry_path.exists() {
        fs::write(&entry_path, entry_template())
            .map_err(|error| format!("{}: {error}", entry_path.display()))?;
    }

    let tsconfig_path = root.join("tsconfig.json");
    if force || !tsconfig_path.exists() {
        fs::write(&tsconfig_path, tsconfig_template())
            .map_err(|error| format!("{}: {error}", tsconfig_path.display()))?;
    }

    println!("initialized sdkt project at {}", root.display());
    Ok(())
}

fn manifest_template(root: &Path) -> String {
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("my_mod")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let name = if name.is_empty() {
        "my_mod".to_string()
    } else {
        name
    };
    format!(
        "[mod]\nid = \"{name}\"\nname = \"{name}\"\n\n[uses]\nplugins = [\"sdk_runtime\"]\n\n[entry]\nfile = \"src/main.ts\"\n"
    )
}

fn entry_template() -> &'static str {
    "import { player } from \"sdk\";\n\nplayer.on_character_changed((ctx) => {\n  oppw4.trace(`current=${ctx.current_character?.id ?? \"none\"}`);\n});\n"
}

fn tsconfig_template() -> &'static str {
    "{\n  \"compilerOptions\": {\n    \"target\": \"ES2020\",\n    \"module\": \"ESNext\",\n    \"moduleResolution\": \"Bundler\",\n    \"strict\": true,\n    \"allowJs\": true,\n    \"checkJs\": true\n  },\n  \"include\": [\"src/**/*.ts\", \"src/**/*.js\", \".sdkt/types/**/*.d.ts\"]\n}\n"
}

#[cfg(test)]
mod tests {
    use std::{env, time::SystemTime};

    use super::*;

    #[test]
    fn init_scaffolds_typescript_project_layout() {
        let root = temp_root("project-init");

        init_project(&root, false).expect("init");

        assert!(root.join("src").join("main.ts").is_file());
        assert!(root.join("tsconfig.json").is_file());
        assert!(root.join("mod.toml").is_file());
        assert!(!root.join("main.js").exists());
        assert!(!root.join("mods").exists());

        let manifest = fs::read_to_string(root.join("mod.toml")).expect("manifest");
        assert!(manifest.contains("file = \"src/main.ts\""));

        let config = fs::read_to_string(root.join(".sdkt").join("config.toml")).expect("config");
        assert!(config.contains("mods_path = \".sdkt/mods\""));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
