use sdk_schema::{
    push_schema_type_descriptor, RegistryEventDescriptor, RegistryModuleSchema,
    RegistryTypeDescriptor, SchemaEntity, SchemaTypeDependencies, SchemaTypeRef,
};

pub(super) fn event_module_schema<T>(
    mut schema: RegistryModuleSchema,
    event_name: &str,
    event_key: &str,
) -> RegistryModuleSchema
where
    T: SchemaEntity + SchemaTypeDependencies + SchemaTypeRef,
{
    add_schema_types::<T>(&mut schema);
    schema.event(RegistryEventDescriptor::new(
        event_name,
        event_key,
        T::schema_type_ref(),
    ))
}

pub(super) fn add_schema_types<T>(schema: &mut RegistryModuleSchema)
where
    T: SchemaTypeDependencies,
{
    let mut descriptors = Vec::<RegistryTypeDescriptor>::new();
    T::collect_schema_types(&mut descriptors);
    for descriptor in descriptors {
        push_schema_type_descriptor(&mut schema.types, descriptor);
    }
}
