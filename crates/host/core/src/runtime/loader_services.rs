use std::{ffi::c_void, sync::OnceLock};

use plugin_abi::{
    HostModuleBaseFn, HostReadMemoryFn, HostRegisterFileProviderFn, HostScanMemoryFn,
    HostWriteMemoryFn, Oppw4FileProvider,
};

static MEMORY: OnceLock<LoaderMemory> = OnceLock::new();
static FILE_PROVIDER_REGISTRAR: OnceLock<LoaderFileProviderRegistrar> = OnceLock::new();

#[derive(Clone, Copy)]
struct LoaderFileProviderRegistrar {
    host_context: usize,
    callback: HostRegisterFileProviderFn,
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

#[derive(Clone, Copy)]
struct LoaderMemory {
    host_context: usize,
    module_base: HostModuleBaseFn,
    read: HostReadMemoryFn,
    write: HostWriteMemoryFn,
    scan: HostScanMemoryFn,
}
