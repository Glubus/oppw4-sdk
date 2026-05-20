use std::{
    ffi::c_void,
    sync::OnceLock,
};

use plugin_abi::{HostRegisterFileProviderFn, Oppw4FileProvider};

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
