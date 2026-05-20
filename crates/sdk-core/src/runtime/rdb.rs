use std::{
    ffi::c_void,
    sync::OnceLock,
};

use plugin_abi::{HostRdbPatchReadFn, HostRegisterRdbPatchProviderFn};

use crate::runtime::ffi::{context_from_raw, CAP_RDB_PATCH};

static RDB_SERVICE: OnceLock<RdbService> = OnceLock::new();

#[derive(Clone, Copy)]
struct RdbService {
    context: usize,
    register_patch_provider: HostRegisterRdbPatchProviderFn,
}

pub(crate) unsafe extern "system" fn host_register_rdb_service(
    _host_context: *mut c_void,
    service_context: *mut c_void,
    register_patch_provider: Option<HostRegisterRdbPatchProviderFn>,
) -> i32 {
    let Some(register_patch_provider) = register_patch_provider else {
        return -1;
    };
    RDB_SERVICE
        .set(RdbService {
            context: service_context as usize,
            register_patch_provider,
        })
        .map(|_| 0)
        .unwrap_or(-2)
}

pub(crate) unsafe extern "system" fn host_register_rdb_patch_provider(
    host_context: *mut c_void,
    provider_context: *mut c_void,
    patch_read: Option<HostRdbPatchReadFn>,
) -> i32 {
    let Some(patch_read) = patch_read else {
        return -1;
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) = context.require_capability(CAP_RDB_PATCH) {
        return code;
    }
    let Some(service) = RDB_SERVICE.get() else {
        return -40;
    };
    unsafe {
        (service.register_patch_provider)(
            service.context as *mut c_void,
            provider_context,
            Some(patch_read),
        )
    }
}
