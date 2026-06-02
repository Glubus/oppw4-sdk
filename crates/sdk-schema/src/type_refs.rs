use crate::{RegistryTypeDescriptor, RegistryTypeRef, SchemaEntity};

pub trait SchemaTypeRef {
    fn schema_type_ref() -> RegistryTypeRef;
}

pub trait SchemaTypeDependencies {
    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>);
}

impl<T> SchemaTypeRef for T
where
    T: SchemaEntity,
{
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Named {
            name: T::schema_entity_name().to_string(),
        }
    }
}

impl<T> SchemaTypeDependencies for T
where
    T: SchemaEntity,
{
    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>) {
        T::collect_schema_types(out);
    }
}

impl SchemaTypeRef for () {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Void
    }
}

impl SchemaTypeDependencies for () {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

impl SchemaTypeRef for bool {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Bool
    }
}

impl SchemaTypeDependencies for bool {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

macro_rules! impl_i64_type_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SchemaTypeRef for $ty {
                fn schema_type_ref() -> RegistryTypeRef {
                    RegistryTypeRef::I64
                }
            }

            impl SchemaTypeDependencies for $ty {
                fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
            }
        )*
    };
}

impl_i64_type_ref!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl SchemaTypeRef for f32 {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::F64
    }
}

impl SchemaTypeDependencies for f32 {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

impl SchemaTypeRef for f64 {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::F64
    }
}

impl SchemaTypeDependencies for f64 {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

impl SchemaTypeRef for String {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::String
    }
}

impl SchemaTypeDependencies for String {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

impl SchemaTypeRef for str {
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::String
    }
}

impl SchemaTypeDependencies for str {
    fn collect_schema_types(_out: &mut Vec<RegistryTypeDescriptor>) {}
}

impl<T> SchemaTypeRef for Option<T>
where
    T: SchemaTypeRef,
{
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Optional {
            inner: Box::new(T::schema_type_ref()),
        }
    }
}

impl<T> SchemaTypeDependencies for Option<T>
where
    T: SchemaTypeDependencies,
{
    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>) {
        T::collect_schema_types(out);
    }
}

impl<T> SchemaTypeRef for Vec<T>
where
    T: SchemaTypeRef,
{
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Array {
            inner: Box::new(T::schema_type_ref()),
        }
    }
}

impl<T> SchemaTypeDependencies for Vec<T>
where
    T: SchemaTypeDependencies,
{
    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>) {
        T::collect_schema_types(out);
    }
}

impl<T> SchemaTypeRef for [T]
where
    T: SchemaTypeRef,
{
    fn schema_type_ref() -> RegistryTypeRef {
        RegistryTypeRef::Array {
            inner: Box::new(T::schema_type_ref()),
        }
    }
}

impl<T> SchemaTypeDependencies for [T]
where
    T: SchemaTypeDependencies,
{
    fn collect_schema_types(out: &mut Vec<RegistryTypeDescriptor>) {
        T::collect_schema_types(out);
    }
}
