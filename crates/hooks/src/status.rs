use std::sync::OnceLock;

static FILE_OPEN_OBSERVER: OnceLock<fn(&str)> = OnceLock::new();

pub fn set_file_open_observer(observer: fn(&str)) -> bool {
    FILE_OPEN_OBSERVER.set(observer).is_ok()
}

pub(crate) fn mark_file_open(path: &str) {
    if let Some(observer) = FILE_OPEN_OBSERVER.get() {
        observer(path);
    }
}
