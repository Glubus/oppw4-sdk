use std::{ffi::c_void, fmt, sync::Arc};

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryModuleSchema {
    pub namespace: String,
    pub import_name: String,
    pub constructible: bool,
    pub functions: Vec<RegistryFunctionDescriptor>,
    pub types: Vec<RegistryTypeDescriptor>,
    #[serde(default)]
    pub extensions: Vec<RegistryTypeExtensionDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryFunctionDescriptor {
    pub name: String,
    pub params: Vec<RegistryParamDescriptor>,
    pub returns: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryParamDescriptor {
    pub name: String,
    pub type_ref: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryTypeDescriptor {
    pub name: String,
    pub constructible: bool,
    pub fields: Vec<RegistryFieldDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryFieldDescriptor {
    pub name: String,
    pub type_ref: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryTypeExtensionDescriptor {
    pub target_type: String,
    pub methods: Vec<RegistryMethodDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryMethodDescriptor {
    pub name: String,
    pub function: String,
    pub returns: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RegistryTypeRef {
    Named { name: String },
    Optional { inner: Box<RegistryTypeRef> },
    Array { inner: Box<RegistryTypeRef> },
    Void,
    Bool,
    I64,
    F64,
    String,
    Json,
}

impl RegistryModuleSchema {
    pub fn new(namespace: impl Into<String>, import_name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            import_name: import_name.into(),
            constructible: false,
            functions: Vec::new(),
            types: Vec::new(),
            extensions: Vec::new(),
        }
    }

    pub fn constructible(mut self, constructible: bool) -> Self {
        self.constructible = constructible;
        self
    }

    pub fn function(mut self, function: RegistryFunctionDescriptor) -> Self {
        self.functions.push(function);
        self
    }

    pub fn type_descriptor(mut self, type_descriptor: RegistryTypeDescriptor) -> Self {
        self.types.push(type_descriptor);
        self
    }

    pub fn extension(mut self, extension: RegistryTypeExtensionDescriptor) -> Self {
        self.extensions.push(extension);
        self
    }
}

impl RegistryFunctionDescriptor {
    pub fn new(name: impl Into<String>, returns: RegistryTypeRef) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            returns,
        }
    }

    pub fn param(mut self, name: impl Into<String>, type_ref: RegistryTypeRef) -> Self {
        self.params.push(RegistryParamDescriptor {
            name: name.into(),
            type_ref,
        });
        self
    }
}

impl RegistryTypeDescriptor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constructible: false,
            fields: Vec::new(),
        }
    }

    pub fn constructible(mut self, constructible: bool) -> Self {
        self.constructible = constructible;
        self
    }

    pub fn field(mut self, name: impl Into<String>, type_ref: RegistryTypeRef) -> Self {
        self.fields.push(RegistryFieldDescriptor {
            name: name.into(),
            type_ref,
        });
        self
    }
}

impl RegistryTypeExtensionDescriptor {
    pub fn new(target_type: impl Into<String>) -> Self {
        Self {
            target_type: target_type.into(),
            methods: Vec::new(),
        }
    }

    pub fn method(mut self, method: RegistryMethodDescriptor) -> Self {
        self.methods.push(method);
        self
    }
}

impl RegistryMethodDescriptor {
    pub fn new(
        name: impl Into<String>,
        function: impl Into<String>,
        returns: RegistryTypeRef,
    ) -> Self {
        Self {
            name: name.into(),
            function: function.into(),
            returns,
        }
    }
}
