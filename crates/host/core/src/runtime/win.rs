use std::ffi::{c_char, c_void};

type Hmodule = *mut c_void;

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(path: *const u16) -> Hmodule;
    fn GetProcAddress(module: Hmodule, name: *const c_char) -> *mut c_void;
}

pub(crate) fn load_library(path: &[u16]) -> Hmodule {
    unsafe { LoadLibraryW(path.as_ptr()) }
}

pub(crate) unsafe fn get_proc_address(module: Hmodule, name: *const c_char) -> *mut c_void {
    GetProcAddress(module, name)
}
