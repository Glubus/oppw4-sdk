use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use crate::{
    log,
    patching::{self, ModAsset, ReplacementSource, VirtualReplacement},
    provider,
};

static STATE: OnceLock<Mutex<Option<SkinPatcherState>>> = OnceLock::new();

#[derive(Clone)]
pub struct SkinPatcherState {
    rdb_root: PathBuf,
    catalog: Vec<rdb::NameHashEntry>,
}

pub fn initialize(rdb_root: PathBuf, catalog: Vec<rdb::NameHashEntry>) {
    let state = STATE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = state.lock() {
        *guard = Some(SkinPatcherState { rdb_root, catalog });
    }
}

pub fn register_asset_replacements(
    archive_name: &str,
    replacements: Vec<AssetReplacement>,
) -> Result<usize, String> {
    let state = current_state()?;
    let mut by_archive_name = HashMap::<String, ReplacementSource>::new();
    for replacement in replacements {
        by_archive_name.insert(
            replacement.target_file_name.to_ascii_lowercase(),
            replacement.source,
        );
    }
    let target_names = by_archive_name
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let rdb_path = state.rdb_root.join(format!("{archive_name}.rdb"));
    let bytes = std::fs::read(&rdb_path).map_err(|error| {
        format!(
            "{archive_name}: failed to read {}: {error}",
            rdb_path.display()
        )
    })?;
    let index = rdb::parse_rdb(&bytes).map_err(|_| format!("{archive_name}: rdb parse failed"))?;
    let scan =
        rdb::scan_archive_names_with_catalog(archive_name, &index, &target_names, &state.catalog);
    let assets = scan
        .files
        .iter()
        .filter_map(|file| {
            let source = by_archive_name
                .get(&file.file_name.to_ascii_lowercase())?
                .clone();
            Some(ModAsset {
                file_name: file.file_name.clone(),
                source,
            })
        })
        .collect::<Vec<_>>();
    let replacements = patching::build_virtualization_table_from_assets(&scan, &assets);
    if replacements.len() != target_names.len() {
        log_unresolved_targets(archive_name, &target_names, &replacements);
    }
    let replacements = patching::attach_mod_file_sizes(replacements)
        .map_err(|error| format!("{archive_name}: failed to read replacement asset: {error}"))?;
    let bin_base_size = std::fs::metadata(state.rdb_root.join(format!("{archive_name}.rdb.bin")))
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    provider::add_runtime_replacements(archive_name, bin_base_size, replacements)
}

#[derive(Clone, Debug)]
pub struct AssetReplacement {
    pub target_file_name: String,
    pub source: ReplacementSource,
}

pub fn mod_source_from_lua(
    lua: &mlua::Lua,
    path: impl AsRef<std::path::Path>,
) -> mlua::Result<ReplacementSource> {
    match lua_api::resolve_mod_file_source(lua, path)? {
        lua_api::ModFileSource::File(path) => Ok(ReplacementSource::File(path)),
        lua_api::ModFileSource::ZipEntry {
            zip_path,
            entry_name,
        } => Ok(ReplacementSource::ZipEntry {
            zip_path,
            entry_name,
        }),
    }
}

fn current_state() -> Result<SkinPatcherState, String> {
    let state = STATE
        .get()
        .ok_or_else(|| "skin_patcher runtime is not initialized".to_string())?;
    let guard = state
        .lock()
        .map_err(|_| "skin_patcher runtime lock failed".to_string())?;
    guard
        .clone()
        .ok_or_else(|| "skin_patcher runtime is not initialized".to_string())
}

fn log_unresolved_targets(
    archive_name: &str,
    target_names: &[&str],
    replacements: &[VirtualReplacement],
) {
    for target in target_names {
        if !replacements
            .iter()
            .any(|replacement| replacement.file_name.eq_ignore_ascii_case(target))
        {
            log::write_line(format!(
                "skin_patcher unresolved {archive_name} target={target}"
            ));
        }
    }
}
