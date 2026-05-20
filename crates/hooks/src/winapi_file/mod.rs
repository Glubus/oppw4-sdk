use std::{
    ffi::{c_char, c_void, CStr},
    mem::size_of,
    sync::{Mutex, OnceLock},
};

mod handles;
mod logs;
mod path;
mod tracker;
mod types;

use crate::{log, win};
use handles::{
    fake_to_handle, returned_virtual_handle, virtual_handle_for_os_handle, VirtualHandle,
};
use logs::{log_create_file_candidate, log_open_virtual, log_patch_read, log_virtual_io};
use path::{hex_preview, read_overlapped_event, read_overlapped_offset, wide_path_to_string};
use tracker::OpenFileTracker;
use types::*;

static ORIGINALS: OnceLock<OriginalFunctions> = OnceLock::new();
static FILE_PROVIDERS: OnceLock<Mutex<Vec<FileProvider>>> = OnceLock::new();
static OPEN_FILES: OnceLock<Mutex<OpenFileTracker>> = OnceLock::new();

pub type ProviderOpenPathFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    path_utf8: *const c_char,
    out_handle: *mut u64,
) -> i32;
pub type ProviderReadFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    buffer: *mut u8,
    bytes_to_read: u32,
    requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32;
pub type ProviderCloseFn =
    unsafe extern "system" fn(provider_context: *mut c_void, handle: u64) -> i32;
pub type ProviderSizeFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    out_size: *mut u64,
) -> i32;
pub type ProviderFileTimeFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    out_filetime: *mut u64,
) -> i32;
pub type ProviderSeekFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    distance: i64,
    move_method: u32,
    out_position: *mut u64,
) -> i32;
pub type ProviderPatchReadFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32;

pub struct FileProviderRegistration<'a> {
    pub plugin_id: Option<&'a CStr>,
    pub provider_context: *mut c_void,
    pub open_path: ProviderOpenPathFn,
    pub read: ProviderReadFn,
    pub close: ProviderCloseFn,
    pub size: ProviderSizeFn,
    pub file_time: Option<ProviderFileTimeFn>,
    pub seek: ProviderSeekFn,
    pub patch_read: Option<ProviderPatchReadFn>,
}

#[derive(Clone, Copy)]
struct FileProvider {
    id: usize,
    provider_context: usize,
    open_path: ProviderOpenPathFn,
    read: ProviderReadFn,
    close: ProviderCloseFn,
    size: ProviderSizeFn,
    file_time: Option<ProviderFileTimeFn>,
    seek: ProviderSeekFn,
    patch_read: Option<ProviderPatchReadFn>,
}

pub fn register_file_provider(provider: FileProviderRegistration<'_>) -> i32 {
    let plugin_id = fixed_plugin_id(provider.plugin_id);
    let registry = FILE_PROVIDERS.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(mut guard) = registry.lock() else {
        return -1;
    };
    let id = guard.len();
    guard.push(FileProvider {
        id,
        provider_context: provider.provider_context as usize,
        open_path: provider.open_path,
        read: provider.read,
        close: provider.close,
        size: provider.size,
        file_time: provider.file_time,
        seek: provider.seek,
        patch_read: provider.patch_read,
    });
    log::write_line(format!(
        "file provider registered: {} index={id}",
        String::from_utf8_lossy(&plugin_id).trim_end_matches('\0')
    ));
    0
}

fn fixed_plugin_id(plugin_id: Option<&CStr>) -> [u8; 64] {
    let mut out = [0u8; 64];
    let Some(plugin_id) = plugin_id else {
        return out;
    };
    let bytes = plugin_id.to_bytes();
    let len = bytes.len().min(out.len().saturating_sub(1));
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

pub fn install_main_module_hooks() {
    let module = win::main_module();
    if module.is_null() {
        log::write_line("hook install skipped: main module not found");
        return;
    }

    let originals = unsafe { install_iat_hooks(module as usize) };
    let installed = count_installed(originals);
    let _ = ORIGINALS.set(originals);
    log::write_line(format!("IAT hooks installed: {installed}"));
}

unsafe fn install_iat_hooks(module: usize) -> OriginalFunctions {
    let mut originals = OriginalFunctions::empty();
    let Some((import_rva, import_size)) = import_directory(module) else {
        log::write_line("hook install skipped: import directory missing");
        return originals;
    };
    if import_rva == 0 || import_size == 0 {
        log::write_line("hook install skipped: empty import directory");
        return originals;
    }

    let mut descriptor = (module + import_rva as usize) as *const ImageImportDescriptor;
    while (*descriptor).name != 0 {
        patch_import_descriptor(module, &*descriptor, &mut originals);
        descriptor = descriptor.add(1);
    }
    originals
}

unsafe fn import_directory(module: usize) -> Option<(u32, u32)> {
    let dos = module as *const u8;
    if std::slice::from_raw_parts(dos, 2) != b"MZ" {
        return None;
    }
    let nt_offset = *(dos.add(0x3c) as *const u32) as usize;
    let nt = dos.add(nt_offset);
    if std::slice::from_raw_parts(nt, 4) != b"PE\0\0" {
        return None;
    }

    let optional_header = nt.add(24);
    let magic = *(optional_header as *const u16);
    if magic != 0x20b {
        return None;
    }

    let data_directories = optional_header.add(112);
    let import_directory = data_directories.add(IMAGE_DIRECTORY_ENTRY_IMPORT * 8);
    let rva = *(import_directory as *const u32);
    let size = *(import_directory.add(4) as *const u32);
    Some((rva, size))
}

unsafe fn patch_import_descriptor(
    module: usize,
    descriptor: &ImageImportDescriptor,
    originals: &mut OriginalFunctions,
) {
    let lookup_rva = if descriptor.original_first_thunk != 0 {
        descriptor.original_first_thunk
    } else {
        descriptor.first_thunk
    };
    if lookup_rva == 0 || descriptor.first_thunk == 0 {
        return;
    }

    let mut lookup = (module + lookup_rva as usize) as *const u64;
    let mut iat = (module + descriptor.first_thunk as usize) as *mut usize;
    while *lookup != 0 {
        let entry = *lookup;
        if entry & IMAGE_ORDINAL_FLAG64 == 0 {
            let import_by_name = (module + entry as usize) as *const u8;
            let name = c_string_at(import_by_name.add(2));
            patch_import_by_name(name, iat, originals);
        }
        lookup = lookup.add(1);
        iat = iat.add(1);
    }
}

unsafe fn patch_import_by_name(name: &str, iat: *mut usize, originals: &mut OriginalFunctions) {
    match name {
        "CreateFileW" => patch_slot(
            iat,
            hooked_create_file_w as *const () as usize,
            |original| {
                originals.create_file_w =
                    Some(std::mem::transmute::<usize, CreateFileWFn>(original));
            },
        ),
        "ReadFile" => patch_slot(iat, hooked_read_file as *const () as usize, |original| {
            originals.read_file = Some(std::mem::transmute::<usize, ReadFileFn>(original));
        }),
        "CloseHandle" => patch_slot(iat, hooked_close_handle as *const () as usize, |original| {
            originals.close_handle = Some(std::mem::transmute::<usize, CloseHandleFn>(original));
        }),
        "GetFileSizeEx" => patch_slot(
            iat,
            hooked_get_file_size_ex as *const () as usize,
            |original| {
                originals.get_file_size_ex =
                    Some(std::mem::transmute::<usize, GetFileSizeExFn>(original));
            },
        ),
        "GetFileTime" => patch_slot(
            iat,
            hooked_get_file_time as *const () as usize,
            |original| {
                originals.get_file_time =
                    Some(std::mem::transmute::<usize, GetFileTimeFn>(original));
            },
        ),
        "GetFileType" => patch_slot(
            iat,
            hooked_get_file_type as *const () as usize,
            |original| {
                originals.get_file_type =
                    Some(std::mem::transmute::<usize, GetFileTypeFn>(original));
            },
        ),
        "SetFilePointerEx" => patch_slot(
            iat,
            hooked_set_file_pointer_ex as *const () as usize,
            |original| {
                originals.set_file_pointer_ex =
                    Some(std::mem::transmute::<usize, SetFilePointerExFn>(original));
            },
        ),
        _ => {}
    }
}

unsafe fn patch_slot<F>(slot: *mut usize, replacement: usize, assign_original: F)
where
    F: FnOnce(usize),
{
    if *slot == replacement {
        return;
    }
    let original = *slot;
    let mut old_protect = 0;
    if !win::make_memory_writable(slot.cast(), size_of::<usize>(), &mut old_protect) {
        return;
    }
    *slot = replacement;
    let _ = win::restore_memory_protection(slot.cast(), size_of::<usize>(), old_protect);
    assign_original(original);
}

unsafe fn c_string_at(ptr: *const u8) -> &'static str {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8_unchecked(bytes)
}

fn count_installed(originals: OriginalFunctions) -> usize {
    [
        originals.create_file_w.is_some(),
        originals.read_file.is_some(),
        originals.close_handle.is_some(),
        originals.get_file_size_ex.is_some(),
        originals.get_file_time.is_some(),
        originals.get_file_type.is_some(),
        originals.set_file_pointer_ex.is_some(),
    ]
    .into_iter()
    .filter(|installed| *installed)
    .count()
}

unsafe extern "system" fn hooked_create_file_w(
    path: Lpcwstr,
    desired_access: Dword,
    share_mode: Dword,
    security_attributes: Lpvoid,
    creation_disposition: Dword,
    flags_and_attributes: Dword,
    template_file: Handle,
) -> Handle {
    let full_path = wide_path_to_string(path);
    if let Some(path) = full_path.as_deref() {
        log_create_file_candidate(path, desired_access, creation_disposition);
    }
    let Some(original) = ORIGINALS
        .get()
        .and_then(|originals| originals.create_file_w)
    else {
        return INVALID_HANDLE_VALUE;
    };
    if desired_access == GENERIC_READ {
        if let Some(path) = full_path.as_deref() {
            if let Some(handle) = open_virtual_fake_handle(path) {
                return handle;
            }
        }
    }

    let handle = original(
        path,
        desired_access,
        share_mode,
        security_attributes,
        creation_disposition,
        flags_and_attributes,
        template_file,
    );
    if let Some(path) = full_path.as_deref() {
        with_open_files(|files| files.track_open(handle, path));
    }
    handle
}

unsafe extern "system" fn hooked_read_file(
    handle: Handle,
    buffer: Lpvoid,
    bytes_to_read: Dword,
    bytes_read: Lpdword,
    overlapped: Lpvoid,
) -> Bool {
    if let Some(virtual_handle) = virtual_handle_for_os_handle(handle) {
        return read_virtual_file(
            virtual_handle,
            buffer,
            bytes_to_read,
            bytes_read,
            overlapped,
        );
    }

    let Some(original) = ORIGINALS.get().and_then(|originals| originals.read_file) else {
        return 0;
    };
    let tracked = tracked_read(handle, overlapped);
    let result = original(handle, buffer, bytes_to_read, bytes_read, overlapped);
    if result != 0 {
        patch_tracked_read(tracked, buffer, bytes_to_read, bytes_read);
    }
    result
}

unsafe extern "system" fn hooked_close_handle(handle: Handle) -> Bool {
    if let Some(virtual_handle) = fake_to_handle(handle) {
        return close_virtual_file(virtual_handle);
    }

    let Some(original) = ORIGINALS.get().and_then(|originals| originals.close_handle) else {
        return 0;
    };
    with_open_files(|files| files.untrack(handle));
    original(handle)
}

unsafe extern "system" fn hooked_get_file_size_ex(handle: Handle, size: *mut LargeInteger) -> Bool {
    if let Some(virtual_handle) = virtual_handle_for_os_handle(handle) {
        return get_virtual_file_size(virtual_handle, size);
    }

    let Some(original) = ORIGINALS
        .get()
        .and_then(|originals| originals.get_file_size_ex)
    else {
        return 0;
    };
    original(handle, size)
}

unsafe extern "system" fn hooked_get_file_time(
    handle: Handle,
    creation_time: Lpvoid,
    last_access_time: Lpvoid,
    last_write_time: Lpvoid,
) -> Bool {
    if let Some(virtual_handle) = virtual_handle_for_os_handle(handle) {
        return unsafe {
            get_virtual_file_time(
                virtual_handle,
                creation_time,
                last_access_time,
                last_write_time,
            )
        };
    }

    let Some(original) = ORIGINALS
        .get()
        .and_then(|originals| originals.get_file_time)
    else {
        return 0;
    };
    original(handle, creation_time, last_access_time, last_write_time)
}

unsafe extern "system" fn hooked_get_file_type(handle: Handle) -> Dword {
    if let Some(virtual_handle) = virtual_handle_for_os_handle(handle) {
        log_virtual_io(format_args!(
            "Virtual TYPE handle=0x{:x} type=disk",
            virtual_handle.as_raw()
        ));
        return FILE_TYPE_DISK;
    }

    let Some(original) = ORIGINALS
        .get()
        .and_then(|originals| originals.get_file_type)
    else {
        return 0;
    };
    original(handle)
}

unsafe extern "system" fn hooked_set_file_pointer_ex(
    handle: Handle,
    distance: LargeInteger,
    new_pointer: *mut LargeInteger,
    move_method: Dword,
) -> Bool {
    if let Some(virtual_handle) = virtual_handle_for_os_handle(handle) {
        return seek_virtual_file(virtual_handle, distance, new_pointer, move_method);
    }

    let Some(original) = ORIGINALS
        .get()
        .and_then(|originals| originals.set_file_pointer_ex)
    else {
        return 0;
    };
    original(handle, distance, new_pointer, move_method)
}

fn open_virtual_fake_handle(path: &str) -> Option<Handle> {
    let path = std::ffi::CString::new(path.as_bytes()).ok()?;
    let mut raw_handle = 0u64;
    let opened = with_providers(|providers| {
        for provider in providers {
            raw_handle = 0;
            let opened = unsafe {
                (provider.open_path)(
                    provider.provider_context as *mut c_void,
                    path.as_ptr(),
                    &mut raw_handle,
                )
            };
            if opened > 0 && raw_handle != 0 {
                return Some(provider_handle(provider.id, raw_handle));
            }
        }
        None
    })??;
    let virtual_handle = VirtualHandle::from_raw(opened);
    let handle = returned_virtual_handle(virtual_handle);
    log_open_virtual(
        path.as_c_str().to_string_lossy().as_ref(),
        handle,
        virtual_handle,
    );
    Some(handle)
}

fn provider_handle(provider_id: usize, raw_handle: u64) -> u64 {
    ((provider_id as u64) << 48) | (raw_handle & 0x0000_ffff_ffff_ffff)
}

fn split_provider_handle(handle: VirtualHandle) -> Option<(usize, u64)> {
    let raw = handle.as_raw();
    Some(((raw >> 48) as usize, raw & 0x0000_ffff_ffff_ffff))
}

fn with_provider_for_handle<T>(
    handle: VirtualHandle,
    action: impl FnOnce(&FileProvider, u64) -> T,
) -> Option<T> {
    let (provider_id, raw_handle) = split_provider_handle(handle)?;
    with_providers(|providers| {
        let provider = providers
            .iter()
            .find(|provider| provider.id == provider_id)?;
        Some(action(provider, raw_handle))
    })?
}

#[allow(dead_code)]
fn old_open_virtual_fake_handle(path: &str) -> Option<Handle> {
    let path = std::ffi::CString::new(path.as_bytes()).ok()?;
    let mut raw_handle = 0u64;
    let opened = with_first_provider(|provider| unsafe {
        (provider.open_path)(
            provider.provider_context as *mut c_void,
            path.as_ptr(),
            &mut raw_handle,
        )
    })?;
    if opened <= 0 || raw_handle == 0 {
        return None;
    }
    let virtual_handle = VirtualHandle::from_raw(raw_handle);
    let handle = returned_virtual_handle(virtual_handle);
    log_open_virtual(
        path.as_c_str().to_string_lossy().as_ref(),
        handle,
        virtual_handle,
    );
    Some(handle)
}

unsafe fn read_virtual_file(
    handle: VirtualHandle,
    buffer: Lpvoid,
    bytes_to_read: Dword,
    bytes_read: Lpdword,
    overlapped: Lpvoid,
) -> Bool {
    if bytes_to_read == 0 {
        if !bytes_read.is_null() {
            *bytes_read = 0;
        }
        log_virtual_io(format_args!(
            "Virtual READ handle=0x{:x} offset=unchanged request=0x0 read=0x0",
            handle.as_raw()
        ));
        return 1;
    }
    if buffer.is_null() {
        return 0;
    }
    let requested_offset = if overlapped.is_null() {
        None
    } else {
        let offset = read_overlapped_offset(overlapped);
        if offset < 0 {
            return 0;
        }
        Some(offset as u64)
    };
    let Some(mut read) = with_provider_for_handle(handle, |provider, raw_handle| {
        let mut read = 0u32;
        let result = unsafe {
            (provider.read)(
                provider.provider_context as *mut c_void,
                raw_handle,
                buffer.cast(),
                bytes_to_read,
                requested_offset.map(|offset| offset as i64).unwrap_or(-1),
                &mut read,
            )
        };
        (result > 0).then_some(read)
    })
    .flatten() else {
        return 0;
    };
    read = read.min(bytes_to_read);
    let buffer = std::slice::from_raw_parts_mut(buffer.cast::<u8>(), read as usize);
    if !bytes_read.is_null() {
        *bytes_read = read;
    }
    let h_event = if overlapped.is_null() {
        0
    } else {
        read_overlapped_event(overlapped)
    };
    let preview_len = (read as usize).min(16);
    let preview = hex_preview(&buffer[..preview_len]);
    log_virtual_io(format_args!(
        "Virtual READ handle=0x{:x} offset={} request=0x{:x} read=0x{:x} hEvent=0x{h_event:x} first={preview}",
        handle.as_raw(),
        requested_offset
            .map(|offset| format!("0x{offset:x}"))
            .unwrap_or_else(|| "fp".to_string()),
        bytes_to_read,
        read
    ));
    1
}

fn close_virtual_file(handle: VirtualHandle) -> Bool {
    let closed = with_provider_for_handle(handle, |provider, raw_handle| unsafe {
        (provider.close)(provider.provider_context as *mut c_void, raw_handle)
    })
    .filter(|closed| *closed > 0)
    .map(|_| 1)
    .unwrap_or(0);
    log_virtual_io(format_args!(
        "Virtual CLOSE handle=0x{:x} closed={closed}",
        handle.as_raw()
    ));
    closed
}

unsafe fn get_virtual_file_size(handle: VirtualHandle, size: *mut LargeInteger) -> Bool {
    if size.is_null() {
        return 0;
    }
    let mut file_size = 0u64;
    let Some(result) = with_provider_for_handle(handle, |provider, raw_handle| unsafe {
        (provider.size)(
            provider.provider_context as *mut c_void,
            raw_handle,
            &mut file_size,
        )
    }) else {
        return 0;
    };
    if result <= 0 {
        return 0;
    }
    *size = file_size as LargeInteger;
    log_virtual_io(format_args!(
        "Virtual SIZE handle=0x{:x} size=0x{file_size:x}",
        handle.as_raw()
    ));
    1
}

unsafe fn get_virtual_file_time(
    handle: VirtualHandle,
    creation_time: Lpvoid,
    last_access_time: Lpvoid,
    last_write_time: Lpvoid,
) -> Bool {
    if let Some((creation, access, write)) = virtual_file_times(handle) {
        write_file_time(creation_time, creation);
        write_file_time(last_access_time, access);
        write_file_time(last_write_time, write);
        log_virtual_io(format_args!(
            "Virtual TIME handle=0x{:x} source=mod",
            handle.as_raw()
        ));
        return 1;
    }

    let mut now = FileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    GetSystemTimeAsFileTime(&mut now);
    write_file_time(creation_time, now);
    write_file_time(last_access_time, now);
    write_file_time(last_write_time, now);
    log_virtual_io(format_args!(
        "Virtual TIME handle=0x{:x} source=system",
        handle.as_raw()
    ));
    1
}

fn virtual_file_times(handle: VirtualHandle) -> Option<(FileTime, FileTime, FileTime)> {
    let mut raw = 0u64;
    let result = with_provider_for_handle(handle, |provider, raw_handle| unsafe {
        let file_time = provider.file_time?;
        Some(file_time(
            provider.provider_context as *mut c_void,
            raw_handle,
            &mut raw,
        ))
    })??;
    if result <= 0 {
        return None;
    }
    let write = FileTime {
        low_date_time: raw as u32,
        high_date_time: (raw >> 32) as u32,
    };
    Some((write, write, write))
}

unsafe fn write_file_time(target: Lpvoid, value: FileTime) {
    if !target.is_null() {
        *target.cast::<FileTime>() = value;
    }
}

unsafe fn seek_virtual_file(
    handle: VirtualHandle,
    distance: LargeInteger,
    new_pointer: *mut LargeInteger,
    move_method: Dword,
) -> Bool {
    let mut position = 0u64;
    let Some(result) = with_provider_for_handle(handle, |provider, raw_handle| unsafe {
        (provider.seek)(
            provider.provider_context as *mut c_void,
            raw_handle,
            distance,
            move_method,
            &mut position,
        )
    }) else {
        return 0;
    };
    if result <= 0 {
        return 0;
    }
    if !new_pointer.is_null() {
        *new_pointer = position as LargeInteger;
    }
    log_virtual_io(format_args!(
        "Virtual SEEK handle=0x{:x} method={move_method} distance=0x{distance:x} new=0x{position:x}",
        handle.as_raw()
    ));
    1
}

fn with_providers<T>(action: impl FnOnce(&[FileProvider]) -> T) -> Option<T> {
    let providers = FILE_PROVIDERS.get()?;
    let guard = providers.lock().ok()?;
    Some(action(&guard))
}

fn with_first_provider<T>(action: impl FnOnce(&FileProvider) -> T) -> Option<T> {
    with_providers(|providers| providers.first().map(action)).flatten()
}

fn with_open_files<T>(action: impl FnOnce(&mut OpenFileTracker) -> T) -> Option<T> {
    let tracker = OPEN_FILES.get_or_init(|| Mutex::new(OpenFileTracker::default()));
    let mut guard = tracker.lock().ok()?;
    Some(action(&mut guard))
}

unsafe fn tracked_read(
    handle: Handle,
    overlapped: Lpvoid,
) -> Option<(String, usize, LargeInteger)> {
    let path = with_open_files(|files| files.path(handle)).flatten()?;
    let offset = if overlapped.is_null() {
        current_file_pointer(handle).unwrap_or(-1)
    } else {
        read_overlapped_offset(overlapped)
    };
    Some((path, handle as usize, offset))
}

unsafe fn patch_tracked_read(
    tracked: Option<(String, usize, LargeInteger)>,
    buffer: Lpvoid,
    bytes_to_read: Dword,
    bytes_read: Lpdword,
) {
    let Some((path, os_handle, offset)) = tracked else {
        return;
    };
    if offset < 0 || buffer.is_null() {
        return;
    }
    let actual_read = if bytes_read.is_null() {
        bytes_to_read as usize
    } else {
        *bytes_read as usize
    };
    if actual_read == 0 {
        return;
    }
    let buffer = std::slice::from_raw_parts_mut(buffer.cast::<u8>(), actual_read);
    dispatch_patch_read(&path, os_handle, offset as u64, buffer);
}

fn dispatch_patch_read(path: &str, os_handle: usize, read_offset: u64, buffer: &mut [u8]) {
    let Ok(path) = std::ffi::CString::new(path.as_bytes()) else {
        return;
    };
    let patched = with_providers(|providers| {
        providers
            .iter()
            .filter_map(|provider| {
                let patch_read = provider.patch_read?;
                Some(unsafe {
                    patch_read(
                        provider.provider_context as *mut c_void,
                        path.as_ptr(),
                        os_handle,
                        read_offset,
                        buffer.as_mut_ptr(),
                        buffer.len(),
                    )
                })
            })
            .sum::<i32>()
    })
    .unwrap_or_default();
    log_patch_read(
        path.as_c_str(),
        os_handle,
        read_offset,
        buffer.len(),
        patched,
    );
}

unsafe fn current_file_pointer(handle: Handle) -> Option<LargeInteger> {
    let original = ORIGINALS
        .get()
        .and_then(|originals| originals.set_file_pointer_ex)?;
    let mut position = 0;
    if original(handle, 0, &mut position, 1) == 0 {
        return None;
    }
    Some(position)
}
