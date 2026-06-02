use std::{ffi::c_void, fmt, sync::Arc};

use sdk_schema::RegistryModuleSchema;

pub type RuntimeModuleInstallFn =
    unsafe extern "system" fn(module_context: *mut c_void, runtime_context: *mut c_void) -> i32;
pub type RegistryModuleInvokeFn =
    Arc<dyn Fn(&str, &str) -> Result<String, String> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct RegistryModuleDescriptor {
    pub provider_id: String,
    pub module_name: String,
    pub module_context: usize,
    pub install: Option<RuntimeModuleInstallFn>,
    pub invoke: Option<RegistryModuleInvokeFn>,
    pub load: RegistryModuleLoad,
    pub schema: Option<RegistryModuleSchema>,
}

impl fmt::Debug for RegistryModuleDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegistryModuleDescriptor")
            .field("provider_id", &self.provider_id)
            .field("module_name", &self.module_name)
            .field("module_context", &self.module_context)
            .field("has_install", &self.install.is_some())
            .field("has_invoke", &self.invoke.is_some())
            .field("load", &self.load)
            .field("schema", &self.schema)
            .finish()
    }
}

#[derive(Clone)]
pub struct RegistryModuleBuilder {
    provider_id: String,
    module_name: String,
    module_context: usize,
    install: Option<RuntimeModuleInstallFn>,
    invoke: Option<RegistryModuleInvokeFn>,
    load: RegistryModuleLoad,
    schema: Option<RegistryModuleSchema>,
}

impl RegistryModuleDescriptor {
    pub fn builder(
        provider_id: impl Into<String>,
        module_name: impl Into<String>,
    ) -> RegistryModuleBuilder {
        RegistryModuleBuilder {
            provider_id: provider_id.into(),
            module_name: module_name.into(),
            module_context: 0,
            install: None,
            invoke: None,
            load: RegistryModuleLoad::WhenPluginRequested,
            schema: None,
        }
    }
}

impl RegistryModuleBuilder {
    pub fn context(mut self, module_context: usize) -> Self {
        self.module_context = module_context;
        self
    }

    pub fn install(mut self, install: RuntimeModuleInstallFn) -> Self {
        self.install = Some(install);
        self
    }

    pub fn invoke(mut self, invoke: RegistryModuleInvokeFn) -> Self {
        self.invoke = Some(invoke);
        self
    }

    pub fn invoke_opt(mut self, invoke: Option<RegistryModuleInvokeFn>) -> Self {
        self.invoke = invoke;
        self
    }

    pub fn load(mut self, load: RegistryModuleLoad) -> Self {
        self.load = load;
        self
    }

    pub fn schema(mut self, schema: RegistryModuleSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn schema_opt(mut self, schema: Option<RegistryModuleSchema>) -> Self {
        self.schema = schema;
        self
    }

    pub fn build(self) -> RegistryModuleDescriptor {
        RegistryModuleDescriptor {
            provider_id: self.provider_id,
            module_name: self.module_name,
            module_context: self.module_context,
            install: self.install,
            invoke: self.invoke,
            load: self.load,
            schema: self.schema,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RegistryModuleLoad {
    #[default]
    WhenPluginRequested,
    Always,
}
