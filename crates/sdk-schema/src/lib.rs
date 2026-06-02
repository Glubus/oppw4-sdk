extern crate self as sdk_schema;

mod collect;
mod entity;
mod schema;
mod type_refs;
mod validation;

pub use collect::push_schema_type_descriptor;
pub use entity::{SchemaAccessorDescriptor, SchemaAccessorKind, SchemaEntity};
pub use schema::{
    RegistryEventDescriptor, RegistryFieldDescriptor, RegistryFunctionDescriptor,
    RegistryMethodDescriptor, RegistryModuleSchema, RegistryMutationDescriptor,
    RegistryParamDescriptor, RegistryTypeDescriptor, RegistryTypeExtensionDescriptor,
    RegistryTypeRef,
};
pub use sdk_schema_derive::{schema, schema_module, SchemaEntity};
pub use type_refs::{SchemaTypeDependencies, SchemaTypeRef};
pub use validation::RegistrySchemaError;
