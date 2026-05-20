use std::sync::atomic::{AtomicUsize, Ordering};

use crate::log;

use super::handles::VirtualHandle;
use super::types::{Dword, Handle};

static CREATE_FILE_LOGS: AtomicUsize = AtomicUsize::new(0);
static PATCH_READ_LOGS: AtomicUsize = AtomicUsize::new(0);
static OPEN_VIRTUAL_LOGS: AtomicUsize = AtomicUsize::new(0);
static VIRTUAL_IO_LOGS: AtomicUsize = AtomicUsize::new(0);

pub(crate) const DEBUG_FILE_IO_LOGS: bool = false;

pub(crate) fn log_create_file_candidate(
    path: &str,
    desired_access: Dword,
    creation_disposition: Dword,
) {
    crate::mark_file_open(path);
    if !DEBUG_FILE_IO_LOGS {
        return;
    }
    if CREATE_FILE_LOGS.load(Ordering::Relaxed) >= 320 {
        return;
    }
    let lower = path.to_ascii_lowercase();
    let interesting = lower.contains("oppw4")
        || lower.contains("op4")
        || lower.contains(".rdb")
        || lower.contains(".g1")
        || lower.contains("0x");
    if !interesting {
        return;
    }
    let index = CREATE_FILE_LOGS.fetch_add(1, Ordering::Relaxed);
    if index < 320 {
        log::write_line(format!(
            "CreateFileW path={path} access=0x{desired_access:x} disposition=0x{creation_disposition:x}"
        ));
    }
}

pub(crate) fn log_open_virtual(path: &str, returned_handle: Handle, handle: VirtualHandle) {
    if !DEBUG_FILE_IO_LOGS {
        return;
    }
    let index = OPEN_VIRTUAL_LOGS.fetch_add(1, Ordering::Relaxed);
    if index < 80 {
        log::write_line(format!(
            "Open virtual {path} returned=0x{:x} provider_handle=0x{:x}",
            returned_handle as usize,
            handle.as_raw()
        ));
    } else if index == 80 {
        log::write_line("Open virtual logs suppressed");
    }
}

pub(crate) fn log_patch_read(
    path: &std::ffi::CStr,
    os_handle: usize,
    read_offset: u64,
    buffer_len: usize,
    patched: i32,
) {
    if patched == 0 || !DEBUG_FILE_IO_LOGS || PATCH_READ_LOGS.load(Ordering::Relaxed) >= 80 {
        return;
    }
    let index = PATCH_READ_LOGS.fetch_add(1, Ordering::Relaxed);
    if index < 80 {
        log::write_line(format!(
            "Provider PATCH path={} handle=0x{os_handle:x} read=0x{read_offset:x}+0x{buffer_len:x} result={patched}",
            path.to_string_lossy()
        ));
    }
}

pub(crate) fn log_virtual_io(args: std::fmt::Arguments<'_>) {
    if !DEBUG_FILE_IO_LOGS {
        return;
    }
    let index = VIRTUAL_IO_LOGS.fetch_add(1, Ordering::Relaxed);
    if index < 512 {
        log::write_line(args);
    } else if index == 512 {
        log::write_line("Virtual IO logs suppressed");
    }
}
