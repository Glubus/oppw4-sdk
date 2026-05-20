use std::ffi::c_void;

use plugin_abi::{HostPluginModVisitorFn, HostPluginModZipVisitorFn};

use super::ApiContext;

pub(crate) unsafe extern "system" fn host_for_each_plugin_mod_zip(
    host_context: *mut c_void,
    visitor: Option<HostPluginModZipVisitorFn>,
    user_context: *mut c_void,
) -> i32 {
    let Some((context, visitor)) = visitor_context(host_context, visitor) else {
        return super::invalid_visitor_error(host_context, visitor);
    };

    for path in super::legacy_mod_paths(&context.mods_root) {
        let result = visitor(user_context, path.as_ptr());
        if result != 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe extern "system" fn host_for_each_plugin_mod(
    host_context: *mut c_void,
    visitor: Option<HostPluginModVisitorFn>,
    user_context: *mut c_void,
) -> i32 {
    let Some((context, visitor)) = visitor_context(host_context, visitor) else {
        return super::invalid_visitor_error(host_context, visitor);
    };

    for mod_entry in super::plugin_mods_for_context(context) {
        let entry = super::plugin_mod_entry(&mod_entry);
        let result = visitor(user_context, &entry);
        if result != 0 {
            return result;
        }
    }
    0
}

unsafe fn visitor_context<T>(
    host_context: *mut c_void,
    visitor: Option<T>,
) -> Option<(&'static ApiContext, T)> {
    Some((host_context.cast::<ApiContext>().as_ref()?, visitor?))
}
