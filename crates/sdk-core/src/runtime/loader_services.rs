use std::{
    ffi::c_void,
    sync::OnceLock,
};

use plugin_abi::{
    HostActiveCharacterFn, HostGameStatusFn, HostModuleBaseFn, HostReadMemoryFn,
    HostRegisterFileProviderFn, HostScanMemoryFn, HostWriteMemoryFn, Oppw4ActiveCharacter,
    Oppw4FileProvider, Oppw4GameStatus,
};

static MEMORY: OnceLock<LoaderMemory> = OnceLock::new();
static FILE_PROVIDER_REGISTRAR: OnceLock<LoaderFileProviderRegistrar> = OnceLock::new();
static GAME_STATUS_READER: OnceLock<LoaderGameStatusReader> = OnceLock::new();
static ACTIVE_CHARACTER_READER: OnceLock<LoaderActiveCharacterReader> = OnceLock::new();

#[derive(Clone, Copy)]
struct LoaderFileProviderRegistrar {
    host_context: usize,
    callback: HostRegisterFileProviderFn,
}

#[derive(Clone, Copy)]
struct LoaderGameStatusReader {
    host_context: usize,
    callback: HostGameStatusFn,
}

#[derive(Clone, Copy)]
struct LoaderActiveCharacterReader {
    host_context: usize,
    callback: HostActiveCharacterFn,
}

pub fn set_file_provider_registrar(
    host_context: *mut c_void,
    callback: HostRegisterFileProviderFn,
) {
    let _ = FILE_PROVIDER_REGISTRAR.set(LoaderFileProviderRegistrar {
        host_context: host_context as usize,
        callback,
    });
}

pub(crate) fn register_file_provider(provider: &Oppw4FileProvider) -> Option<i32> {
    let registrar = FILE_PROVIDER_REGISTRAR.get()?;
    Some(unsafe { (registrar.callback)(registrar.host_context as *mut c_void, provider) })
}

pub fn set_memory(
    host_context: *mut c_void,
    module_base: HostModuleBaseFn,
    read: HostReadMemoryFn,
    write: HostWriteMemoryFn,
    scan: HostScanMemoryFn,
) {
    let _ = MEMORY.set(LoaderMemory {
        host_context: host_context as usize,
        module_base,
        read,
        write,
        scan,
    });
}

pub(crate) fn module_base() -> Option<usize> {
    let memory = MEMORY.get()?;
    Some(unsafe { (memory.module_base)(memory.host_context as *mut c_void) })
}

pub(crate) fn read_memory(address: usize, out: *mut u8, len: usize) -> Option<i32> {
    let memory = MEMORY.get()?;
    Some(unsafe { (memory.read)(memory.host_context as *mut c_void, address, out, len) })
}

pub(crate) fn write_memory(address: usize, bytes: *const u8, len: usize) -> Option<i32> {
    let memory = MEMORY.get()?;
    Some(unsafe { (memory.write)(memory.host_context as *mut c_void, address, bytes, len) })
}

pub(crate) fn scan_memory(pattern: *const u8, mask: *const u8, len: usize) -> Option<usize> {
    let memory = MEMORY.get()?;
    Some(unsafe { (memory.scan)(memory.host_context as *mut c_void, pattern, mask, len) })
}

pub fn set_game_status_reader(host_context: *mut c_void, callback: HostGameStatusFn) {
    let _ = GAME_STATUS_READER.set(LoaderGameStatusReader {
        host_context: host_context as usize,
        callback,
    });
}

pub(crate) fn read_game_status(out_status: &mut Oppw4GameStatus) -> Option<i32> {
    let reader = GAME_STATUS_READER.get()?;
    Some(unsafe { (reader.callback)(reader.host_context as *mut c_void, out_status) })
}

pub fn set_active_character_reader(host_context: *mut c_void, callback: HostActiveCharacterFn) {
    let _ = ACTIVE_CHARACTER_READER.set(LoaderActiveCharacterReader {
        host_context: host_context as usize,
        callback,
    });
}

pub(crate) fn read_active_character(out: &mut Oppw4ActiveCharacter) -> Option<i32> {
    let reader = ACTIVE_CHARACTER_READER.get()?;
    Some(unsafe { (reader.callback)(reader.host_context as *mut c_void, out) })
}
#[derive(Clone, Copy)]
struct LoaderMemory {
    host_context: usize,
    module_base: HostModuleBaseFn,
    read: HostReadMemoryFn,
    write: HostWriteMemoryFn,
    scan: HostScanMemoryFn,
}
