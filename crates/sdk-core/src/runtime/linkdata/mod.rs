mod api;
mod provider;
mod registry;
mod state;
mod virtual_file;

use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use registry::LinkDataRegistry;

static REGISTRY: OnceLock<Mutex<LinkDataRegistry>> = OnceLock::new();

pub(super) fn initialize(game_root: &Path) {
    let _ = REGISTRY.set(Mutex::new(LinkDataRegistry::new(game_root.to_path_buf())));
    provider::register();
}

fn with_registry<T>(action: impl FnOnce(&mut LinkDataRegistry) -> T) -> Option<T> {
    let registry = REGISTRY.get()?;
    let mut guard = registry.lock().ok()?;
    Some(action(&mut guard))
}

pub(crate) use api::{host_patch_linkdata_row, host_replace_linkdata_entry};
