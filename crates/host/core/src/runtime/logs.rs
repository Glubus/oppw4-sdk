mod writer;

use std::{
    collections::HashMap,
    ffi::CStr,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::manifest::sanitize_plugin_id;
use writer::SessionLogWriter;

use super::time;

static ROUTER: OnceLock<Mutex<LogRouter>> = OnceLock::new();

pub(crate) fn initialize(session_stamp: Option<String>, mod_log_root: PathBuf) {
    let _ = ROUTER.set(Mutex::new(LogRouter::new(session_stamp, mod_log_root)));
}

pub(crate) fn register(plugin_id: String, log_root: PathBuf) {
    if let Some(router) = ROUTER.get() {
        router
            .lock()
            .expect("plugin log router lock")
            .register(plugin_id, log_root);
    }
}

pub(crate) fn write(plugin_id: &CStr, message: &CStr) {
    if let Some(router) = ROUTER.get() {
        let _ = router
            .lock()
            .expect("plugin log router lock")
            .route_plugin(plugin_id, message);
    }
}

pub(crate) fn write_mod(mod_id: &str, message: &str) {
    if let Some(router) = ROUTER.get() {
        let _ = router
            .lock()
            .expect("log router lock")
            .route_mod(mod_id, message);
    }
}

struct LogRouter {
    session_stamp: String,
    plugin_roots: HashMap<String, PathBuf>,
    plugin_writers: HashMap<String, SessionLogWriter>,
    mod_log_root: PathBuf,
    mod_writers: HashMap<String, SessionLogWriter>,
}

impl LogRouter {
    fn new(session_stamp: Option<String>, mod_log_root: PathBuf) -> Self {
        Self {
            session_stamp: session_stamp.unwrap_or_else(time::file_timestamp),
            plugin_roots: HashMap::new(),
            plugin_writers: HashMap::new(),
            mod_log_root,
            mod_writers: HashMap::new(),
        }
    }

    fn register(&mut self, plugin_id: String, log_root: PathBuf) {
        let plugin_id = sanitize_plugin_id(&plugin_id);
        let _ = fs::create_dir_all(&log_root);
        self.plugin_roots.insert(plugin_id, log_root);
    }

    fn route_plugin(&mut self, plugin_id: &CStr, message: &CStr) -> std::io::Result<()> {
        let plugin_id = sanitize_plugin_id(&plugin_id.to_string_lossy());
        let message = message.to_string_lossy();
        let session_stamp = self.session_stamp.clone();
        self.plugin_writer_for(&plugin_id)?
            .write(&message, &session_stamp)
    }

    fn route_mod(&mut self, mod_id: &str, message: &str) -> std::io::Result<()> {
        let mod_id = sanitize_plugin_id(mod_id);
        let session_stamp = self.session_stamp.clone();
        self.mod_writer_for(&mod_id)?.write(message, &session_stamp)
    }

    fn plugin_writer_for(&mut self, plugin_id: &str) -> std::io::Result<&mut SessionLogWriter> {
        if !self.plugin_writers.contains_key(plugin_id) {
            let root = self.plugin_log_root_for(plugin_id);
            fs::create_dir_all(&root)?;
            self.plugin_writers
                .insert(plugin_id.to_string(), SessionLogWriter::new(root));
        }
        Ok(self
            .plugin_writers
            .get_mut(plugin_id)
            .expect("plugin log writer was inserted"))
    }

    fn mod_writer_for(&mut self, mod_id: &str) -> std::io::Result<&mut SessionLogWriter> {
        if !self.mod_writers.contains_key(mod_id) {
            let root = self.mod_log_root.join(mod_id);
            fs::create_dir_all(&root)?;
            self.mod_writers
                .insert(mod_id.to_string(), SessionLogWriter::new(root));
        }
        Ok(self
            .mod_writers
            .get_mut(mod_id)
            .expect("mod log writer was inserted"))
    }

    fn plugin_log_root_for(&self, plugin_id: &str) -> PathBuf {
        self.plugin_roots
            .get(plugin_id)
            .cloned()
            .unwrap_or_else(|| PathBuf::from("plugins").join(plugin_id).join("logs"))
    }
}

#[cfg(test)]
mod tests;
