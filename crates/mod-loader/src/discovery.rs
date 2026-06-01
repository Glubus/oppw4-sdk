use std::{fs, io::Read, path::Path};

use crate::{parse_mod_manifest, DiscoveredMod, ModSource};

pub fn discover_mods(root: &Path) -> Vec<DiscoveredMod> {
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

fn collect_directory_mod(path: &Path, mods: &mut Vec<DiscoveredMod>) {
    let manifest_path = path.join("mod.toml");
    let Ok(text) = fs::read_to_string(manifest_path) else {
        return;
    };
    let Ok(manifest) = parse_mod_manifest(&text) else {
        return;
    };
    mods.push(DiscoveredMod {
        manifest,
        source: ModSource::Directory(path.to_path_buf()),
    });
}

fn collect_zip_mod(path: &Path, mods: &mut Vec<DiscoveredMod>) {
    let Ok((manifest_entry, text)) = read_zip_manifest(path) else {
        return;
    };
    let Ok(manifest) = parse_mod_manifest(&text) else {
        return;
    };
    mods.push(DiscoveredMod {
        manifest,
        source: ModSource::Zip {
            path: path.to_path_buf(),
            root: zip_mod_root(&manifest_entry),
        },
    });
}

fn zip_mod_root(manifest_entry: &str) -> String {
    manifest_entry
        .rsplit_once('/')
        .map(|(root, _)| format!("{root}/"))
        .unwrap_or_default()
}

fn read_zip_manifest(path: &Path) -> std::io::Result<(String, String)> {
    if let Ok(text) = read_zip_text(path, "mod.toml") {
        return Ok(("mod.toml".to_string(), text));
    }

    let manifest_entry = find_nested_mod_manifest(path)?;
    let text = read_zip_text(path, &manifest_entry)?;
    Ok((manifest_entry, text))
}

fn find_nested_mod_manifest(path: &Path) -> std::io::Result<String> {
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
    match matches.as_slice() {
        [manifest] => Ok(manifest.clone()),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "zip must contain exactly one mod.toml",
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_mod_root_is_empty_for_root_manifest() {
        assert_eq!(zip_mod_root("mod.toml"), "");
    }

    #[test]
    fn zip_mod_root_preserves_nested_prefix() {
        assert_eq!(zip_mod_root("nested/mod.toml"), "nested/");
    }

    #[test]
    fn nested_mod_manifest_rejects_parent_paths() {
        assert!(!is_nested_mod_manifest("../mod.toml"));
        assert!(!is_nested_mod_manifest("nested/../mod.toml"));
    }
}
