use crate::runtime::debug;

mod r#unsafe;

pub(crate) use r#unsafe::host_debug_enabled;

fn debug_enabled() -> i32 {
    debug::enabled() as i32
}
