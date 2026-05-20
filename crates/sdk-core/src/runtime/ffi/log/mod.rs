mod r#unsafe;

use std::ffi::CStr;

use crate::runtime::logs;

pub(crate) use r#unsafe::host_log;

fn write_log(plugin_id: &CStr, message: &CStr) {
    logs::write(plugin_id, message);
}
