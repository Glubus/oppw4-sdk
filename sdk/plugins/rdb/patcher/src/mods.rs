use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use plugin_sdk::zip::{is_zip_path, zip_file_entries};

use crate::patching::{ModAsset, ReplacementSource};

pub struct ModRepository {
    root: PathBuf,
    zip_paths: Option<Vec<PathBuf>>,
}

impl ModRepository {
    pub fn with_zip_paths(root: PathBuf, zip_paths: Vec<PathBuf>) -> Self {
        Self {
            root,
            zip_paths: Some(zip_paths),
        }
    }

    pub fn archive_assets(&self, archive_name: &str) -> Vec<ModAsset> {
        let mut assets = HashMap::new();
        for zip_path in self.zip_paths() {
            self.collect_zip_archive_assets(archive_name, &zip_path, &mut assets);
        }
        self.collect_loose_archive_assets(archive_name, &mut assets);

        let mut assets: Vec<_> = assets.into_values().collect();
        assets.sort_by_key(|asset| asset.file_name.to_ascii_lowercase());
        assets
    }

    fn collect_loose_archive_assets(
        &self,
        archive_name: &str,
        assets: &mut HashMap<String, ModAsset>,
    ) {
        for archive_root in self.loose_archive_roots(archive_name) {
            collect_loose_archive_root_assets(&archive_root, assets);
        }
    }

    fn loose_archive_roots(&self, archive_name: &str) -> Vec<PathBuf> {
        named_loose_archive_roots(&self.root, archive_name)
    }

    fn collect_zip_archive_assets(
        &self,
        archive_name: &str,
        zip_path: &Path,
        assets: &mut HashMap<String, ModAsset>,
    ) {
        let Ok(entries) = zip_file_entries(zip_path) else {
            return;
        };

        for entry in entries {
            let entry_name = entry.name().to_string();
            let Some(file_name) = archive_entry_file_name(archive_name, &entry_name) else {
                continue;
            };
            insert_asset(
                assets,
                ModAsset {
                    file_name,
                    source: ReplacementSource::ZipEntry {
                        zip_path: zip_path.to_path_buf(),
                        entry_name,
                    },
                },
            );
        }
    }

    fn zip_paths(&self) -> Vec<PathBuf> {
        if let Some(paths) = &self.zip_paths {
            return paths.clone();
        }
        let mut paths = Vec::new();
        collect_zip_paths(&self.root, &mut paths);
        paths.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
        paths
    }
}

fn collect_loose_archive_root_assets(archive_root: &Path, assets: &mut HashMap<String, ModAsset>) {
    let Some(entries) = read_directory(archive_root) else {
        return;
    };

    for entry in entries.flatten() {
        collect_loose_file_asset(entry, assets);
    }
}

fn collect_loose_file_asset(entry: fs::DirEntry, assets: &mut HashMap<String, ModAsset>) {
    let Some(file_type) = entry.file_type().ok() else {
        return;
    };
    if !file_type.is_file() {
        return;
    }
    let file_name = entry.file_name().to_string_lossy().into_owned();
    insert_asset(
        assets,
        ModAsset {
            file_name,
            source: ReplacementSource::File(entry.path()),
        },
    );
}

fn named_loose_archive_roots(root: &Path, archive_name: &str) -> Vec<PathBuf> {
    let Some(entries) = read_directory(root) else {
        return Vec::new();
    };

    let mut roots: Vec<_> = entries
        .flatten()
        .filter_map(|entry| named_loose_archive_root(entry, archive_name))
        .collect();
    roots.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    roots
}

fn named_loose_archive_root(entry: fs::DirEntry, archive_name: &str) -> Option<PathBuf> {
    if !is_directory_entry(&entry) || is_config_directory(&entry) {
        return None;
    }
    if entry_name_eq(&entry, archive_name) {
        return None;
    }

    let archive_root = entry.path().join(archive_name);
    archive_root.is_dir().then_some(archive_root)
}

fn insert_asset(assets: &mut HashMap<String, ModAsset>, asset: ModAsset) {
    assets.insert(asset.file_name.to_ascii_lowercase(), asset);
}

fn collect_zip_paths(root: &Path, paths: &mut Vec<PathBuf>) {
    let Some(entries) = read_directory(root) else {
        return;
    };
    for entry in entries.flatten() {
        collect_zip_path_entry(entry, paths);
    }
}

fn read_directory(root: &Path) -> Option<fs::ReadDir> {
    fs::read_dir(root).ok()
}

fn collect_zip_path_entry(entry: fs::DirEntry, paths: &mut Vec<PathBuf>) {
    let Some(file_type) = entry.file_type().ok() else {
        return;
    };
    let path = entry.path();

    if file_type.is_dir() {
        collect_zip_paths_from_directory(&entry, &path, paths);
    } else if file_type.is_file() && is_zip_file(&path) {
        paths.push(path);
    }
}

fn is_directory_entry(entry: &fs::DirEntry) -> bool {
    entry.file_type().is_ok_and(|file_type| file_type.is_dir())
}

fn collect_zip_paths_from_directory(entry: &fs::DirEntry, path: &Path, paths: &mut Vec<PathBuf>) {
    if is_config_directory(entry) {
        return;
    }
    collect_zip_paths(path, paths);
}

fn is_config_directory(entry: &fs::DirEntry) -> bool {
    entry_name_eq(entry, "_oppw4")
}

fn entry_name_eq(entry: &fs::DirEntry, expected: &str) -> bool {
    entry
        .file_name()
        .to_string_lossy()
        .eq_ignore_ascii_case(expected)
}

fn is_zip_file(path: &Path) -> bool {
    is_zip_path(path)
}

fn archive_entry_file_name(archive_name: &str, entry_name: &str) -> Option<String> {
    let parts: Vec<_> = entry_name
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    let archive_index = parts
        .iter()
        .position(|part| part.eq_ignore_ascii_case(archive_name))?;
    let file_name = parts.get(archive_index + 1..)?.last()?;
    (!file_name.is_empty()).then(|| (*file_name).to_string())
}
