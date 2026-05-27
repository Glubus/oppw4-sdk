use std::ffi::{c_void, CStr};

use plugin_abi::{optional_cstr, Oppw4LogEntry};

pub(crate) unsafe extern "system" fn host_log(
    _host_context: *mut c_void,
    entry: *const Oppw4LogEntry,
) {
    let Some((plugin_id, message)) = log_entry_parts(entry) else {
        return;
    };
    super::write_log(plugin_id, message);
}

unsafe fn log_entry_parts(entry: *const Oppw4LogEntry) -> Option<(&'static CStr, &'static CStr)> {
    let entry = entry.as_ref()?;
    Some((
        optional_cstr(entry.plugin_id)?,
        optional_cstr(entry.message)?,
    ))
}
