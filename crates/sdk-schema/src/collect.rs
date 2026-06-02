use crate::RegistryTypeDescriptor;

pub fn push_schema_type_descriptor(
    out: &mut Vec<RegistryTypeDescriptor>,
    descriptor: RegistryTypeDescriptor,
) {
    if out.iter().all(|known| known.name != descriptor.name) {
        out.push(descriptor);
    }
}
