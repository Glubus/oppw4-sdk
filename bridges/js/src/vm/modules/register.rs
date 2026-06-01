use std::ffi::c_void;

use rquickjs::Ctx;

use crate::{module::JsModuleRef, vm::error};

pub(super) fn register_plugin_module(
    ctx: Ctx<'_>,
    module: &JsModuleRef<'_>,
) -> rquickjs::Result<()> {
    let result = unsafe {
        (module.register)(
            module.context as *mut c_void,
            (&ctx as *const Ctx<'_>).cast_mut().cast(),
        )
    };
    if result != 0 {
        return Err(error::js(
            "Rust",
            "JsModule",
            format!(
                "js module register failed plugin={} module={} result={result}",
                module.plugin_id, module.module_name
            ),
        ));
    }
    Ok(())
}
