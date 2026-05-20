use std::{env, fs, path::Path, process};

use lua_api::{parse_mod_manifest, LuaModManifest};

const MOD_MANIFEST_FILE: &str = "mod.toml";

fn main() {
    let path = match parse_args(env::args().skip(1)) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    match validate_manifest(&path) {
        Ok(manifest) => {
            println!(
                "ok id={} name={} entry_lua={}",
                manifest.id, manifest.name, manifest.entry_lua
            );
            print_list("uses_plugins", &manifest.uses_plugins);
        }
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    let Some(path) = args.next() else {
        return Err(format!("usage: mod-manifest <path-to-{MOD_MANIFEST_FILE}>"));
    };
    if args.next().is_some() {
        return Err(format!("usage: mod-manifest <path-to-{MOD_MANIFEST_FILE}>"));
    }
    Ok(path)
}

fn validate_manifest(path: &str) -> Result<LuaModManifest, String> {
    let path = Path::new(path);
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let manifest = parse_mod_manifest(&text)
        .map_err(|error| format!("invalid {}: {error:?}", path.display()))?;
    validate_directory_entry(path, &manifest)?;
    Ok(manifest)
}

fn validate_directory_entry(path: &Path, manifest: &LuaModManifest) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let entry = parent.join(&manifest.entry_lua);
    if entry.is_file() {
        Ok(())
    } else {
        Err(format!("entry lua file is missing: {}", entry.display()))
    }
}

fn print_list(label: &str, values: &[String]) {
    if values.is_empty() {
        println!("{label}: []");
    } else {
        println!("{label}: {}", values.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_manifest_path() {
        assert_eq!(
            parse_args(["mod.toml".to_string()].into_iter()).expect("args"),
            "mod.toml"
        );
    }

    #[test]
    fn rejects_missing_manifest_path() {
        assert_eq!(
            parse_args(Vec::<String>::new().into_iter()).expect_err("usage"),
            "usage: mod-manifest <path-to-mod.toml>"
        );
    }

    #[test]
    fn validates_existing_entry_file() {
        let root =
            std::env::temp_dir().join(format!("oppw4-mod-manifest-tool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("mod.lua"), "return true").expect("entry lua");
        let manifest = LuaModManifest {
            id: "test_mod".to_string(),
            name: "Test Mod".to_string(),
            uses_plugins: Vec::new(),
            entry_lua: "mod.lua".to_string(),
        };

        assert_eq!(
            validate_directory_entry(&root.join("mod.toml"), &manifest),
            Ok(())
        );
        let _ = fs::remove_dir_all(&root);
    }
}
