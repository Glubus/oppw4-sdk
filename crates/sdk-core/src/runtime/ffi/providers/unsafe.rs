use std::ffi::c_void;

use plugin_abi::{optional_cstr, Oppw4FileProvider};

use crate::runtime::ffi::context::{context_from_raw, CAP_FILES_VIRTUALIZE};

pub(crate) unsafe extern "system" fn host_register_file_provider(
    host_context: *mut c_void,
    provider: *const Oppw4FileProvider,
) -> i32 {
    let Some(provider) = provider.as_ref() else {
        return -1;
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) =
        context.require_capability_for_cstr(optional_cstr(provider.plugin_id), CAP_FILES_VIRTUALIZE)
    {
        return code;
    }
    super::register_file_provider(provider, optional_cstr(provider.plugin_id))
}
