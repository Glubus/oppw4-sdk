use serde::{Deserialize, Serialize};

use crate::{validation::validate_schema, RegistrySchemaError};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryModuleSchema {
    pub namespace: String,
    pub import_name: String,
    pub constructible: bool,
    pub functions: Vec<RegistryFunctionDescriptor>,
    pub types: Vec<RegistryTypeDescriptor>,
    #[serde(default)]
    pub extensions: Vec<RegistryTypeExtensionDescriptor>,
    #[serde(default)]
    pub events: Vec<RegistryEventDescriptor>,
    #[serde(default)]
    pub mutations: Vec<RegistryMutationDescriptor>,
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
pub struct RegistryEventDescriptor {
    pub name: String,
    pub key: String,
    pub payload: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryMutationDescriptor {
    pub name: String,
    pub key: String,
    pub payload: RegistryTypeRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryMethodDescriptor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation: Option<String>,
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
            events: Vec::new(),
            mutations: Vec::new(),
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

    pub fn event(mut self, event: RegistryEventDescriptor) -> Self {
        self.events.push(event);
        self
    }

    pub fn mutation(mut self, mutation: RegistryMutationDescriptor) -> Self {
        self.mutations.push(mutation);
        self
    }

    pub fn validate_contract(&self) -> Result<(), RegistrySchemaError> {
        validate_schema(self)
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

impl RegistryEventDescriptor {
    pub fn new(name: impl Into<String>, key: impl Into<String>, payload: RegistryTypeRef) -> Self {
        Self {
            name: name.into(),
            key: key.into(),
            payload,
        }
    }
}

impl RegistryMutationDescriptor {
    pub fn new(name: impl Into<String>, key: impl Into<String>, payload: RegistryTypeRef) -> Self {
        Self {
            name: name.into(),
            key: key.into(),
            payload,
        }
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
            function: Some(function.into()),
            mutation: None,
            returns,
        }
    }

    pub fn mutation(
        name: impl Into<String>,
        mutation: impl Into<String>,
        returns: RegistryTypeRef,
    ) -> Self {
        Self {
            name: name.into(),
            function: None,
            mutation: Some(mutation.into()),
            returns,
        }
    }
}
