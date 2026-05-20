mod r#unsafe;

pub(crate) use r#unsafe::{
    host_module_base, host_read_memory, host_scan_memory, host_write_memory,
};

fn module_base() -> usize {
    if let Some(module_base) = crate::runtime::loader_services::module_base() {
        return module_base;
    }
    hooks::module_base()
}

fn read_memory(address: usize, out: *mut u8, len: usize) -> i32 {
    if let Some(result) = crate::runtime::loader_services::read_memory(address, out, len) {
        return result;
    }
    unsafe { hooks::read_memory(address, out, len) }
}

fn write_memory(address: usize, bytes: *const u8, len: usize) -> i32 {
    if let Some(result) = crate::runtime::loader_services::write_memory(address, bytes, len) {
        return result;
    }
    unsafe { hooks::write_memory(address, bytes, len) }
}

fn scan_memory(pattern: *const u8, mask: *const u8, len: usize) -> usize {
    if let Some(address) = crate::runtime::loader_services::scan_memory(pattern, mask, len) {
        return address;
    }
    unsafe { hooks::scan_memory(pattern, mask, len) }
}
