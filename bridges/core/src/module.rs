use std::ffi::c_void;

pub type RuntimeModuleInstallFn =
    unsafe extern "system" fn(module_context: *mut c_void, runtime_context: *mut c_void) -> i32;

#[derive(Clone, Debug)]
pub struct RegistryModuleDescriptor {
    pub provider_id: String,
    pub module_name: String,
    pub module_context: usize,
    pub install: Option<RuntimeModuleInstallFn>,
    pub load: RegistryModuleLoad,
    pub schema: Option<RegistryModuleSchema>,
}

#[derive(Clone, Debug)]
pub struct RegistryModuleBuilder {
    provider_id: String,
    module_name: String,
    module_context: usize,
    install: Option<RuntimeModuleInstallFn>,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryModuleSchema {
    pub namespace: String,
    pub import_name: String,
    pub constructible: bool,
    pub functions: Vec<RegistryFunctionDescriptor>,
    pub types: Vec<RegistryTypeDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryFunctionDescriptor {
    pub name: String,
    pub params: Vec<RegistryParamDescriptor>,
    pub returns: RegistryTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryParamDescriptor {
    pub name: String,
    pub type_ref: RegistryTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryTypeDescriptor {
    pub name: String,
    pub constructible: bool,
    pub fields: Vec<RegistryFieldDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryFieldDescriptor {
    pub name: String,
    pub type_ref: RegistryTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryTypeRef {
    Named(String),
    Optional(Box<RegistryTypeRef>),
    Array(Box<RegistryTypeRef>),
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
