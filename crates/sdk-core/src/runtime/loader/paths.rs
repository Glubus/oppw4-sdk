use std::path::{Path, PathBuf};

pub(super) fn mods_root(game_root: &Path) -> PathBuf {
    game_root.join("mods")
}

pub(super) fn data_root(game_root: &Path) -> PathBuf {
    game_root.join("oppw4-data")
}

pub(super) fn path_to_wide(path: &Path) -> Vec<u16> {
    let mut wide = path.to_string_lossy().encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide
}
