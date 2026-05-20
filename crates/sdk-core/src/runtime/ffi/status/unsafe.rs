use std::ffi::c_void;

pub(crate) unsafe extern "system" fn host_debug_enabled(_host_context: *mut c_void) -> i32 {
    super::debug_enabled()
}
