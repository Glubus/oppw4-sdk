use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    io::SeekFrom,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use plugin_sdk::{HostApi, PluginError, VirtualFileProvider};

use crate::{
    log,
    patching::{ReplacementSource, VirtualHandle, VirtualManager, VirtualReplacement},
    rdb_tracker::{self, TrackedFileKind},
};

const FILETIME_TICKS_PER_SECOND: u64 = 10_000_000;
const WINDOWS_TO_UNIX_EPOCH_SECONDS: u64 = 11_644_473_600;

static RUNTIME: OnceLock<Mutex<Option<VirtualManager>>> = OnceLock::new();
static SOURCES: OnceLock<Mutex<HashMap<u64, ReplacementSource>>> = OnceLock::new();

pub fn register_replacements(
    host: HostApi<'_>,
    plugin_id: &'static str,
    replacements: Vec<VirtualReplacement>,
) -> i32 {
    let count = replacements.len();
    let runtime = RUNTIME.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = runtime.lock() else {
        return -10;
    };
    *guard = Some(VirtualManager::new(replacements));
    drop(guard);

    let provider = VirtualFileProvider::new(plugin_id, open_path, read, close, size)
        .file_time(file_time)
        .seek(seek)
        .patch_read(patch_read);
    let registered = match host.rdb().register_virtual_provider(provider) {
        Ok(()) => 0,
        Err(PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };
    log::write_line(format!(
        "skin_patcher rdb virtual provider result={registered} replacements={count}"
    ));
    registered
}

pub fn add_runtime_replacements(
    archive_name: &str,
    bin_base_size: u64,
    replacements: Vec<VirtualReplacement>,
) -> Result<usize, String> {
    if replacements.is_empty() {
        return Ok(0);
    }
    let runtime = RUNTIME
        .get()
        .ok_or_else(|| "skin_patcher virtual provider is not initialized".to_string())?;
    let mut guard = runtime
        .lock()
        .map_err(|_| "skin_patcher virtual provider lock failed".to_string())?;
    let manager = guard
        .as_mut()
        .ok_or_else(|| "skin_patcher virtual manager is not initialized".to_string())?;
    let base_offset = manager
        .virtual_archive_size(archive_name)
        .unwrap_or(bin_base_size);
    let replacements =
        crate::patching::assign_virtual_bin_offsets(replacements, archive_name, base_offset);
    let count = replacements.len();
    manager.append_replacements(replacements);
    Ok(count)
}

unsafe extern "system" fn open_path(
    _context: *mut c_void,
    path_utf8: *const c_char,
    out_handle: *mut u64,
) -> i32 {
    if path_utf8.is_null() || out_handle.is_null() {
        return -1;
    }
    let path = CStr::from_ptr(path_utf8).to_string_lossy();
    let Some((handle, source)) = with_manager(|manager| {
        manager
            .open_by_path_fragment_with_replacement(file_name_or_path(&path).as_ref())
            .ok()
            .flatten()
            .or_else(|| {
                manager
                    .open_by_path_fragment_with_replacement(&path)
                    .ok()
                    .flatten()
            })
    })
    .flatten() else {
        return 0;
    };
    remember_source(handle, source.source);
    *out_handle = handle.as_raw();
    1
}

unsafe extern "system" fn read(
    _context: *mut c_void,
    handle: u64,
    buffer: *mut u8,
    bytes_to_read: u32,
    requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32 {
    if buffer.is_null() {
        return -1;
    }
    let handle = VirtualHandle::from_raw(handle);
    let buffer = std::slice::from_raw_parts_mut(buffer, bytes_to_read as usize);
    let Some(read) = with_manager(|manager| {
        if requested_offset >= 0
            && manager
                .seek(handle, SeekFrom::Start(requested_offset as u64))
                .is_err()
        {
            return None;
        }
        manager.read(handle, buffer).ok()
    })
    .flatten() else {
        return 0;
    };
    if !out_bytes_read.is_null() {
        *out_bytes_read = read as u32;
    }
    1
}

unsafe extern "system" fn close(_context: *mut c_void, handle: u64) -> i32 {
    let handle = VirtualHandle::from_raw(handle);
    let closed = with_manager(|manager| manager.close(handle)).unwrap_or(false);
    if closed {
        forget_source(handle);
        1
    } else {
        0
    }
}

unsafe extern "system" fn size(_context: *mut c_void, handle: u64, out_size: *mut u64) -> i32 {
    if out_size.is_null() {
        return -1;
    }
    let handle = VirtualHandle::from_raw(handle);
    let Some(size) = with_manager(|manager| manager.size(handle).ok()).flatten() else {
        return 0;
    };
    *out_size = size;
    1
}

unsafe extern "system" fn file_time(
    _context: *mut c_void,
    handle: u64,
    out_filetime: *mut u64,
) -> i32 {
    if out_filetime.is_null() {
        return -1;
    }
    let handle = VirtualHandle::from_raw(handle);
    let Some(time) = source(handle)
        .and_then(|source| source.modified_time())
        .and_then(system_time_to_filetime)
    else {
        return 0;
    };
    *out_filetime = time;
    1
}

unsafe extern "system" fn seek(
    _context: *mut c_void,
    handle: u64,
    distance: i64,
    move_method: u32,
    out_position: *mut u64,
) -> i32 {
    let position = match move_method {
        0 => SeekFrom::Start(distance.max(0) as u64),
        1 => SeekFrom::Current(distance),
        2 => SeekFrom::End(distance),
        _ => return -1,
    };
    let handle = VirtualHandle::from_raw(handle);
    let Some(position) = with_manager(|manager| manager.seek(handle, position).ok()).flatten()
    else {
        return 0;
    };
    if !out_position.is_null() {
        *out_position = position;
    }
    1
}

unsafe extern "system" fn patch_read(
    _context: *mut c_void,
    path_utf8: *const c_char,
    _os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32 {
    if path_utf8.is_null() || buffer.is_null() {
        return -1;
    }
    let path = CStr::from_ptr(path_utf8).to_string_lossy();
    let Some(tracked) = rdb_tracker::tracked_read_from_path(&path) else {
        return 0;
    };
    let buffer = std::slice::from_raw_parts_mut(buffer, len);
    match tracked.kind {
        TrackedFileKind::Index => {
            let patched = with_manager(|manager| {
                manager.patch_archive_index_external_flags(
                    &tracked.archive_name,
                    read_offset,
                    buffer,
                )
            })
            .unwrap_or_default();
            if patched > 0 {
                log::write_line(format!(
                    "{} PATCH {}: read=0x{read_offset:x}+0x{:x} fields={patched}",
                    tracked.kind.label(),
                    tracked.archive_name,
                    len
                ));
            }
            patched as i32
        }
        TrackedFileKind::Data => {
            log_data_read_hits(&tracked.archive_name, read_offset, len);
            0
        }
    }
}

fn file_name_or_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn with_manager<T>(action: impl FnOnce(&mut VirtualManager) -> T) -> Option<T> {
    let runtime = RUNTIME.get()?;
    let mut guard = runtime.lock().ok()?;
    let manager = guard.as_mut()?;
    Some(action(manager))
}

fn remember_source(handle: VirtualHandle, source: ReplacementSource) {
    let sources = SOURCES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = sources.lock() {
        guard.insert(handle.as_raw(), source);
    }
}

fn forget_source(handle: VirtualHandle) {
    let Some(sources) = SOURCES.get() else {
        return;
    };
    if let Ok(mut guard) = sources.lock() {
        guard.remove(&handle.as_raw());
    }
}

fn source(handle: VirtualHandle) -> Option<ReplacementSource> {
    let sources = SOURCES.get()?;
    let guard = sources.lock().ok()?;
    guard.get(&handle.as_raw()).cloned()
}

fn system_time_to_filetime(time: SystemTime) -> Option<u64> {
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = duration
        .as_secs()
        .checked_add(WINDOWS_TO_UNIX_EPOCH_SECONDS)?;
    seconds
        .checked_mul(FILETIME_TICKS_PER_SECOND)?
        .checked_add((duration.subsec_nanos() / 100) as u64)
}

fn log_data_read_hits(archive_name: &str, read_offset: u64, read_len: usize) {
    let hits = with_manager(|manager| {
        manager
            .data_read_hits(archive_name, read_offset, read_len)
            .into_iter()
            .map(|replacement| {
                (
                    replacement.file_name.clone(),
                    replacement.hash,
                    replacement.original_bin_offset.unwrap_or_default(),
                    replacement.mod_size.unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for (file_name, hash, original_offset, mod_size) in hits {
        log::write_line(format!(
            "RDB BIN HIT {archive_name}: read=0x{read_offset:x}+0x{read_len:x} file={file_name} hash=0x{hash:08x} bin_offset=0x{original_offset:x} mod_size=0x{mod_size:x}"
        ));
    }
}
