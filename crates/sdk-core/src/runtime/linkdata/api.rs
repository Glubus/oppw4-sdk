use std::{
    ffi::{c_void, CString},
    sync::OnceLock,
};

use plugin_abi::{
    HostPatchLinkDataRowFn, HostReplaceLinkDataEntryFn, Oppw4LinkDataEntryPatch,
    Oppw4LinkDataRowPatch,
};

use crate::runtime::ffi::{context_from_raw, CAP_LINKDATA_PATCH};

static LINKDATA_PROVIDER: OnceLock<LinkDataProvider> = OnceLock::new();

#[derive(Clone, Copy)]
struct LinkDataProvider {
    context: usize,
    replace_entry: HostReplaceLinkDataEntryFn,
    patch_row: HostPatchLinkDataRowFn,
}

pub(crate) unsafe extern "system" fn host_register_linkdata_provider(
    _host_context: *mut c_void,
    provider_context: *mut c_void,
    replace_entry: Option<HostReplaceLinkDataEntryFn>,
    patch_row: Option<HostPatchLinkDataRowFn>,
) -> i32 {
    let (Some(replace_entry), Some(patch_row)) = (replace_entry, patch_row) else {
        return -1;
    };
    LINKDATA_PROVIDER
        .set(LinkDataProvider {
            context: provider_context as usize,
            replace_entry,
            patch_row,
        })
        .map(|_| 0)
        .unwrap_or(-2)
}

pub(crate) unsafe extern "system" fn host_replace_linkdata_entry(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataEntryPatch,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let Some(patch_ref) = patch.as_ref() else {
        return -1;
    };
    if let Err(code) = context.require_capability_for_cstr(
        plugin_abi::optional_cstr(patch_ref.plugin_id),
        CAP_LINKDATA_PATCH,
    ) {
        return code;
    }
    let Some(provider) = LINKDATA_PROVIDER.get() else {
        return -40;
    };
    unsafe { (provider.replace_entry)(provider.context as *mut c_void, patch) }
}

pub(crate) fn replace_entry_from_runtime(
    plugin_id: &str,
    file: u32,
    entry: u32,
    payload: &[u8],
) -> Result<(), String> {
    let Some(provider) = LINKDATA_PROVIDER.get() else {
        return Err("linkdata provider is not registered".to_string());
    };
    let plugin_id = CString::new(plugin_id)
        .map_err(|_| "linkdata patch plugin id contains an interior nul byte".to_string())?;
    let patch = Oppw4LinkDataEntryPatch {
        plugin_id: plugin_id.as_ptr(),
        file,
        entry,
        payload: payload.as_ptr(),
        payload_len: payload.len(),
    };
    let code = unsafe { (provider.replace_entry)(provider.context as *mut c_void, &patch) };
    if code == 0 {
        Ok(())
    } else {
        Err(format!("replace_linkdata_entry failed code={code}"))
    }
}

pub(crate) unsafe extern "system" fn host_patch_linkdata_row(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataRowPatch,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let Some(patch_ref) = patch.as_ref() else {
        return -1;
    };
    if let Err(code) = context.require_capability_for_cstr(
        plugin_abi::optional_cstr(patch_ref.plugin_id),
        CAP_LINKDATA_PATCH,
    ) {
        return code;
    }
    let Some(provider) = LINKDATA_PROVIDER.get() else {
        return -40;
    };
    unsafe { (provider.patch_row)(provider.context as *mut c_void, patch) }
}
