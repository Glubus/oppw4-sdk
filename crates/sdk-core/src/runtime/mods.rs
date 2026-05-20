use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn list_legacy_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_legacy_paths(root, &mut paths);
    paths.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    paths
}

fn collect_legacy_paths(root: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            if !path.join("mod.toml").is_file() && !is_config_directory(&path) {
                paths.push(path);
            }
        } else if file_type.is_file() && is_zip_file(&path) {
            paths.push(path);
        }
    }
}

fn is_config_directory(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("_oppw4"))
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn legacy_paths_skip_declarative_mod_dirs() {
        let root = temp_root("legacy-mods");
        fs::create_dir_all(root.join("skin").join("CharacterEditor")).expect("skin dir");
        fs::create_dir_all(root.join("aura")).expect("aura dir");
        fs::write(root.join("aura").join("mod.toml"), []).expect("mod manifest");
        fs::write(root.join("loose.zip"), []).expect("zip");

        let paths = list_legacy_paths(&root);

        assert_eq!(paths, vec![root.join("loose.zip"), root.join("skin")]);
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }
}
