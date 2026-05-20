use std::ffi::c_void;

pub type Oppw4LuaRegisterFn =
    unsafe extern "system" fn(module_context: *mut c_void, lua_context: *mut c_void) -> i32;
