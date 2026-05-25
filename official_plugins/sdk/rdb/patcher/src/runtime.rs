use std::{collections::HashSet, path::PathBuf};

use plugin_sdk::{zip::read_zip_entry_from_reader, HostApi};

use crate::{log, mods::ModRepository, patching, provider, state, LEGACY_NAME_HASH_CATALOG_ZIP};

const ARCHIVES: [&str; 8] = [
    "CharacterEditor",
    "FieldEditor4",
    "KIDSSystemResource",
    "MaterialEditor",
    "RRPreview",
    "ScreenLayout",
    "SequenceEditor",
    "system",
];

pub fn initialize(host: HostApi<'_>) -> i32 {
    let Some(game_root) = host.paths().game_root() else {
        log::write_line("skin_patcher: missing game_root");
        return -3;
    };
    let Some(mods_root) = host.paths().mods_root() else {
        log::write_line("skin_patcher: missing mods root");
        return -4;
    };
    let config_root = host
        .paths()
        .config_root()
        .unwrap_or_else(|| mods_root.join("_oppw4"));
    let paths = RuntimePaths::from_roots(game_root, mods_root, config_root);
    log_paths(&paths);

    let catalog = load_name_catalog(&paths);
    state::initialize(paths.rdb_root.clone(), catalog.clone());
    let legacy_mod_paths = host
        .mods()
        .legacy_paths()
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    log::write_line(format!("legacy mod paths: {}", legacy_mod_paths.len()));
    let replacements = scan_known_archives(&paths, &catalog, legacy_mod_paths);
    let replacement_count = replacements.len();
    let registered = provider::register_replacements(host, "sdk_rdb", replacements);
    log::write_line(format!(
        "skin_patcher registered replacements result={registered} count={}",
        replacement_count
    ));
    if registered < 0 {
        registered
    } else {
        0
    }
}

struct RuntimePaths {
    mods_root: PathBuf,
    config_root: PathBuf,
    rdb_root: PathBuf,
}

impl RuntimePaths {
    fn from_roots(game_root: PathBuf, mods_root: PathBuf, config_root: PathBuf) -> Self {
        Self {
            config_root,
            mods_root,
            rdb_root: game_root
                .join("File")
                .join("CMN")
                .join("AssetRelease")
                .join("Retail"),
        }
    }
}

fn log_paths(paths: &RuntimePaths) {
    log::write_line(format!("mods root: {}", paths.mods_root.display()));
    log::write_line(format!("config root: {}", paths.config_root.display()));
    log::write_line(format!("rdb root: {}", paths.rdb_root.display()));
}

fn load_name_catalog(paths: &RuntimePaths) -> Vec<rdb::NameHashEntry> {
    let override_path = paths.config_root.join("name_hash_catalog.txt");
    if let Ok(bytes) = std::fs::read(&override_path) {
        let catalog = rdb::parse_name_hash_catalog(&bytes);
        log::write_line(format!(
            "name catalog override: entries={} path={}",
            catalog.len(),
            override_path.display()
        ));
        return catalog;
    }

    let catalog = load_embedded_name_catalog();
    log::write_line(format!(
        "name catalog embedded: entries={} compressed=0x{:x}",
        catalog.len(),
        LEGACY_NAME_HASH_CATALOG_ZIP.len()
    ));
    catalog
}

fn load_embedded_name_catalog() -> Vec<rdb::NameHashEntry> {
    let cursor = std::io::Cursor::new(LEGACY_NAME_HASH_CATALOG_ZIP);
    let bytes = match read_zip_entry_from_reader(cursor, "name_hash_catalog.txt") {
        Ok(bytes) => bytes,
        Err(error) => {
            log::write_line(format!("embedded name catalog read failed: {error}"));
            return Vec::new();
        }
    };
    rdb::parse_name_hash_catalog(&bytes)
}

fn scan_known_archives(
    paths: &RuntimePaths,
    catalog: &[rdb::NameHashEntry],
    legacy_mod_paths: Vec<PathBuf>,
) -> Vec<patching::VirtualReplacement> {
    let mut replacements = Vec::new();
    let mods = ModRepository::with_zip_paths(paths.mods_root.clone(), legacy_mod_paths);
    for archive in ARCHIVES {
        replacements.extend(scan_archive(paths, &mods, archive, catalog));
    }
    let mut attached = match patching::attach_mod_file_sizes(replacements) {
        Ok(replacements) => replacements,
        Err(error) => {
            log::write_line(format!("virtual table size attach failed: {error}"));
            Vec::new()
        }
    };
    let disabled_hashes = load_disabled_hashes(paths);
    if !disabled_hashes.is_empty() {
        let before = attached.len();
        attached.retain(|replacement| !disabled_hashes.contains(&replacement.hash));
        log::write_line(format!(
            "disabled replacements applied: {}",
            before.saturating_sub(attached.len())
        ));
    }
    for archive in ARCHIVES {
        let bin_path = paths.rdb_root.join(format!("{archive}.rdb.bin"));
        let Ok(metadata) = std::fs::metadata(&bin_path) else {
            continue;
        };
        attached = patching::assign_virtual_bin_offsets(attached, archive, metadata.len());
    }
    attached
}

fn load_disabled_hashes(paths: &RuntimePaths) -> HashSet<u32> {
    let path = paths.config_root.join("disabled_hashes.txt");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashSet::new();
    };

    text.lines()
        .filter_map(|line| {
            let raw = line
                .split_once('#')
                .map(|(value, _)| value)
                .unwrap_or(line)
                .trim()
                .trim_end_matches(',');
            let hex = raw.strip_prefix("0x").unwrap_or(raw);
            (!hex.is_empty())
                .then(|| u32::from_str_radix(hex, 16).ok())
                .flatten()
        })
        .collect()
}

fn scan_archive(
    paths: &RuntimePaths,
    mods: &ModRepository,
    archive: &str,
    catalog: &[rdb::NameHashEntry],
) -> Vec<patching::VirtualReplacement> {
    let rdb_path = paths.rdb_root.join(format!("{archive}.rdb"));
    let assets = mods.archive_assets(archive);
    if assets.is_empty() {
        log::write_line(format!("{archive}: no runtime mod assets"));
        return Vec::new();
    }
    let names: Vec<_> = assets
        .iter()
        .map(|asset| asset.file_name.as_str())
        .collect();
    let Ok(bytes) = std::fs::read(&rdb_path) else {
        log::write_line(format!(
            "{archive}: rdb not readable: {}",
            rdb_path.display()
        ));
        return Vec::new();
    };
    let Ok(index) = rdb::parse_rdb(&bytes) else {
        log::write_line(format!("{archive}: rdb parse failed"));
        return Vec::new();
    };

    let scan = rdb::scan_archive_names_with_catalog(archive, &index, &names, catalog);
    let counts = scan.counts();
    log::write_line(format!(
        "{archive}: files={} matched={} hash_missing={} unresolved={}",
        counts.total, counts.matched, counts.hash_missing, counts.unresolved_names
    ));
    patching::build_virtualization_table_from_assets(&scan, &assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_contains_known_law_replacement_hashes() {
        let catalog = load_embedded_name_catalog();

        assert!(catalog.iter().any(|entry| {
            entry.hash == 0x359b9672
                && entry
                    .name
                    .eq_ignore_ascii_case("800_294_face_law_dressrosa_External_00.g1t")
        }));
    }
}
