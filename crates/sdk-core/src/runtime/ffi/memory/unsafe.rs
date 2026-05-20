use std::ffi::c_void;

use crate::runtime::ffi::context::{
    context_from_raw, CAP_MEMORY_READ, CAP_MEMORY_SCAN, CAP_MEMORY_WRITE,
};

pub(crate) unsafe extern "system" fn host_module_base(_host_context: *mut c_void) -> usize {
    super::module_base()
}

pub(crate) unsafe extern "system" fn host_read_memory(
    host_context: *mut c_void,
    address: usize,
    out: *mut u8,
    len: usize,
) -> i32 {
    if let Err(code) = require_capability(host_context, CAP_MEMORY_READ) {
        return code;
    }
    super::read_memory(address, out, len)
}

pub(crate) unsafe extern "system" fn host_write_memory(
    host_context: *mut c_void,
    address: usize,
    bytes: *const u8,
    len: usize,
) -> i32 {
    if let Err(code) = require_capability(host_context, CAP_MEMORY_WRITE) {
        return code;
    }
    super::write_memory(address, bytes, len)
}

pub(crate) unsafe extern "system" fn host_scan_memory(
    host_context: *mut c_void,
    pattern: *const u8,
    mask: *const u8,
    len: usize,
) -> usize {
    if require_capability(host_context, CAP_MEMORY_SCAN).is_err() {
        return 0;
    }
    super::scan_memory(pattern, mask, len)
}

unsafe fn require_capability(host_context: *mut c_void, capability: &str) -> Result<(), i32> {
    context_from_raw(host_context)?.require_capability(capability)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ffi::ApiContext;

    #[test]
    fn read_memory_requires_capability() {
        let mut context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );
        let mut out: [u8; 0] = [];

        let result = unsafe {
            host_read_memory(
                (&mut context as *mut ApiContext).cast(),
                0,
                out.as_mut_ptr(),
                0,
            )
        };

        assert_eq!(result, -22);
    }

    #[test]
    fn scan_memory_requires_capability() {
        let mut context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );
        let pattern: [u8; 0] = [];
        let mask: [u8; 0] = [];

        let result = unsafe {
            host_scan_memory(
                (&mut context as *mut ApiContext).cast(),
                pattern.as_ptr(),
                mask.as_ptr(),
                0,
            )
        };

        assert_eq!(result, 0);
    }
}
