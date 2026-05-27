use std::sync::Arc;

use sdk_bridge::{RegistryModuleLoad, RegistryModuleSchema};

pub type JsModuleInvoke = Arc<dyn Fn(&str, &str) -> Result<String, String> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct JsModule {
    pub plugin_id: String,
    pub module_name: String,
    pub context: usize,
    pub register: unsafe extern "system" fn(
        module_context: *mut std::ffi::c_void,
        js: *mut std::ffi::c_void,
    ) -> i32,
    pub load: RegistryModuleLoad,
    pub schema: Option<RegistryModuleSchema>,
    pub invoke: Option<JsModuleInvoke>,
}
