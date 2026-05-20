use std::{
    ffi::{CStr, CString},
    path::PathBuf,
};

pub fn cstring_lossy(value: impl AsRef<str>) -> CString {
    plugin_abi::cstring_lossy(value)
}

pub(crate) fn path_from_cstr(value: *const std::ffi::c_char) -> Option<PathBuf> {
    if value.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    (!value.is_empty()).then(|| PathBuf::from(value))
}
