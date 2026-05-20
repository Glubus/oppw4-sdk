use std::{
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ModFingerprint {
    manifest_modified: Option<SystemTime>,
    script_modified: Option<SystemTime>,
}

pub(super) fn start_hot_reload_worker(mods_root: PathBuf) {
    let _ = thread::Builder::new()
        .name("oppw4_lua_hot_reload".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(500));
            super::with_host(|host| {
                if host.mods_root() == mods_root {
                    host.reload_changed_directory_mods();
                }
            });
        });
}

pub(super) fn mod_fingerprint(mod_entry: &lua_api::LuaMod) -> Option<ModFingerprint> {
    let lua_api::ModSource::Directory(root) = &mod_entry.source else {
        return None;
    };
    Some(ModFingerprint {
        manifest_modified: modified_time(&root.join("mod.toml")),
        script_modified: modified_time(&root.join(mod_entry.manifest.entry_lua.replace('/', "\\"))),
    })
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
}
