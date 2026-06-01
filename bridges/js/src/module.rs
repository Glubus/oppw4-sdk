use sdk_bridge::{
    RegistryModuleDescriptor, RegistryModuleInvokeFn, RegistryModuleLoad, RegistryModuleSchema,
    RuntimeModuleInstallFn,
};

pub type JsModuleInvoke = RegistryModuleInvokeFn;

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

#[derive(Clone, Copy)]
pub struct JsModuleRef<'a> {
    pub plugin_id: &'a str,
    pub module_name: &'a str,
    pub context: usize,
    pub register: RuntimeModuleInstallFn,
    pub load: RegistryModuleLoad,
    pub schema: Option<&'a RegistryModuleSchema>,
    pub invoke: Option<&'a JsModuleInvoke>,
}

impl<'a> JsModuleRef<'a> {
    pub fn from_descriptor(descriptor: &'a RegistryModuleDescriptor) -> Option<Self> {
        Some(Self {
            plugin_id: descriptor.provider_id.as_str(),
            module_name: descriptor.module_name.as_str(),
            context: descriptor.module_context,
            register: descriptor.install?,
            load: descriptor.load,
            schema: descriptor.schema.as_ref(),
            invoke: descriptor.invoke.as_ref(),
        })
    }
}
