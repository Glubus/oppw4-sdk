use std::collections::HashMap;

use super::types::{Handle, INVALID_HANDLE_VALUE};

#[derive(Default)]
pub(crate) struct OpenFileTracker {
    paths: HashMap<usize, String>,
}

impl OpenFileTracker {
    pub(crate) fn track_open(&mut self, handle: Handle, path: &str) {
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return;
        }
        self.paths.insert(handle as usize, path.to_string());
    }

    pub(crate) fn untrack(&mut self, handle: Handle) {
        self.paths.remove(&(handle as usize));
    }

    pub(crate) fn path(&self, handle: Handle) -> Option<String> {
        self.paths.get(&(handle as usize)).cloned()
    }
}
