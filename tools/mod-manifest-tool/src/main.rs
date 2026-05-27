use std::{env, fs, path::Path, process};

use plugin_sdk::{parse_mod_manifest, ModManifest, MOD_MANIFEST_FILE};

fn main() {
    let args = match parse_args(env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    match validate_manifest(&args.path, args.check_entry) {
        Ok(manifest) => {
            println!(
                "ok id={} name={} entry={}",
                manifest.id, manifest.name, manifest.entry.path
            );
            print_list("uses_plugins", &manifest.uses_plugins);
        }
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Args {
    path: String,
    check_entry: bool,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut path = None;
    let mut check_entry = true;
    for arg in args {
        if arg == "--manifest-only" {
            check_entry = false;
        } else if path.replace(arg).is_some() {
            return Err(usage());
        }
    }
    let Some(path) = path else {
        return Err(usage());
    };
    Ok(Args { path, check_entry })
}

fn usage() -> String {
    format!("usage: mod-manifest [--manifest-only] <path-to-{MOD_MANIFEST_FILE}>")
}

fn validate_manifest(path: &str, check_entry: bool) -> Result<ModManifest, String> {
    let path = Path::new(path);
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let manifest = parse_mod_manifest(&text)
        .map_err(|error| format!("invalid {}: {error:?}", path.display()))?;
    if check_entry {
        validate_directory_entry(path, &manifest)?;
    }
    Ok(manifest)
}

fn validate_directory_entry(path: &Path, manifest: &ModManifest) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let entry = parent.join(&manifest.entry.path);
    if entry.is_file() {
        Ok(())
    } else {
        Err(format!("entry file is missing: {}", entry.display()))
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
            Args {
                path: "mod.toml".to_string(),
                check_entry: true,
            }
        );
    }

    #[test]
    fn parses_manifest_only_flag() {
        assert_eq!(
            parse_args(["--manifest-only".to_string(), "mod.toml".to_string()].into_iter())
                .expect("args"),
            Args {
                path: "mod.toml".to_string(),
                check_entry: false,
            }
        );
    }

    #[test]
    fn rejects_missing_manifest_path() {
        assert_eq!(
            parse_args(Vec::<String>::new().into_iter()).expect_err("usage"),
            "usage: mod-manifest [--manifest-only] <path-to-mod.toml>"
        );
    }

    #[test]
    fn validates_existing_entry_file() {
        let root =
            std::env::temp_dir().join(format!("oppw4-mod-manifest-tool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("main.mod"), "").expect("entry file");
        let manifest = ModManifest {
            id: "test_mod".to_string(),
            name: "Test Mod".to_string(),
            uses_plugins: Vec::new(),
            entry: plugin_sdk::ModEntry {
                path: "main.mod".to_string(),
            },
        };

        assert_eq!(
            validate_directory_entry(&root.join("mod.toml"), &manifest),
            Ok(())
        );
        let _ = fs::remove_dir_all(&root);
    }
}
