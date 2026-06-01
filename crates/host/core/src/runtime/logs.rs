mod writer;

use std::{
    collections::{hash_map::Entry, HashMap},
    ffi::CStr,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::manifest::sanitize_plugin_id;
use writer::SessionLogWriter;

use super::time;

static ROUTER: OnceLock<LogQueue> = OnceLock::new();

pub(crate) fn initialize(session_stamp: Option<String>, mod_log_root: PathBuf) {
    let _ = ROUTER.set(LogQueue::spawn(LogRouter::new(session_stamp, mod_log_root)));
}

pub(crate) fn register(plugin_id: String, log_root: PathBuf) {
    if let Some(router) = ROUTER.get() {
        router.register(plugin_id, log_root);
    }
}

pub(crate) fn write(plugin_id: &CStr, message: &CStr) {
    if let Some(router) = ROUTER.get() {
        router.write_plugin(
            plugin_id.to_string_lossy().into_owned(),
            message.to_string_lossy().into_owned(),
        );
    }
}

pub(crate) fn write_mod(mod_id: &str, message: &str) {
    if let Some(router) = ROUTER.get() {
        router.write_mod(mod_id.to_string(), message.to_string());
    }
}

struct LogQueue {
    router: Mutex<LogRouter>,
}

impl LogQueue {
    fn spawn(router: LogRouter) -> Self {
        Self {
            router: Mutex::new(router),
        }
    }

    fn register(&self, plugin_id: String, log_root: PathBuf) {
        if let Ok(mut router) = self.router.lock() {
            router.register(plugin_id, log_root);
        }
    }

    fn write_plugin(&self, plugin_id: String, message: String) {
        if let Ok(mut router) = self.router.lock() {
            let _ = router.route_plugin_text(&plugin_id, &message);
        }
    }

    fn write_mod(&self, mod_id: String, message: String) {
        if let Ok(mut router) = self.router.lock() {
            let _ = router.route_mod(&mod_id, &message);
        }
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

    fn route_plugin_text(&mut self, plugin_id: &str, message: &str) -> std::io::Result<()> {
        let plugin_id = sanitize_plugin_id(plugin_id);
        let session_stamp = self.session_stamp.clone();
        self.plugin_writer_for(&plugin_id)?
            .write(message, &session_stamp)
    }

    #[cfg(test)]
    fn route_plugin(&mut self, plugin_id: &CStr, message: &CStr) -> std::io::Result<()> {
        let message = message.to_string_lossy();
        self.route_plugin_text(&plugin_id.to_string_lossy(), &message)
    }

    fn route_mod(&mut self, mod_id: &str, message: &str) -> std::io::Result<()> {
        let mod_id = sanitize_plugin_id(mod_id);
        let session_stamp = self.session_stamp.clone();
        self.mod_writer_for(&mod_id)?.write(message, &session_stamp)
    }

    fn plugin_writer_for(&mut self, plugin_id: &str) -> std::io::Result<&mut SessionLogWriter> {
        let plugin_roots = &self.plugin_roots;
        match self.plugin_writers.entry(plugin_id.to_string()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let root = plugin_roots
                    .get(plugin_id)
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from("plugins").join(plugin_id).join("logs"));
                fs::create_dir_all(&root)?;
                Ok(entry.insert(SessionLogWriter::new(root)))
            }
        }
    }

    fn mod_writer_for(&mut self, mod_id: &str) -> std::io::Result<&mut SessionLogWriter> {
        let mod_log_root = &self.mod_log_root;
        match self.mod_writers.entry(mod_id.to_string()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let root = mod_log_root.join(mod_id);
                fs::create_dir_all(&root)?;
                Ok(entry.insert(SessionLogWriter::new(root)))
            }
        }
    }
}

#[cfg(test)]
mod tests;
