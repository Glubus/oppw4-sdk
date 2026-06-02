use crate::{RegistryTypeDescriptor, RegistryTypeRef};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaAccessorKind {
    Getter,
    Setter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaAccessorDescriptor {
    pub kind: SchemaAccessorKind,
    pub method_name: String,
    pub value_field_name: String,
    pub value_type: RegistryTypeRef,
    pub payload_type_name: Option<String>,
}

pub trait SchemaEntity {
    fn schema_entity_name() -> &'static str;

    fn schema_type_descriptor() -> RegistryTypeDescriptor;

    fn schema_accessors() -> Vec<SchemaAccessorDescriptor> {
        Vec::new()
    }

    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>) {
        crate::push_schema_type_descriptor(out, Self::schema_type_descriptor());
    }
}
