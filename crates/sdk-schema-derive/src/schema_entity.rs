use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput};

use crate::{
    field_attrs::{parse_owned_fields, SchemaField},
    shared::{parse_name_value_bool_attr, parse_name_value_string_attr, pascal_case},
};

pub(crate) fn expand_schema_entity(input: DeriveInput) -> syn::Result<TokenStream> {
    let ident = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let entity_name = schema_name(&input.attrs)?.unwrap_or_else(|| ident.to_string());
    let constructible = schema_constructible(&input.attrs)?.unwrap_or(false);

    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new_spanned(
            ident,
            "SchemaEntity can only be derived for structs",
        ));
    };
    let fields = parse_owned_fields(data.fields)?;

    expand_entity_impl(
        quote!(#ident #type_generics),
        quote!(#impl_generics),
        where_clause,
        entity_name,
        constructible,
        fields,
    )
}

pub(crate) fn expand_entity_impl(
    entity_ty: TokenStream,
    impl_generics: TokenStream,
    where_clause: Option<&syn::WhereClause>,
    entity_name: String,
    constructible: bool,
    fields: Vec<SchemaField>,
) -> syn::Result<TokenStream> {
    let descriptor_fields = fields.iter().map(|field| {
        let name = &field.schema_name;
        let ty = &field.ty;
        quote! {
            descriptor = descriptor.field(
                #name,
                <#ty as ::sdk_schema::SchemaTypeRef>::schema_type_ref(),
            );
        }
    });
    let dependency_collectors = fields.iter().map(|field| {
        let ty = &field.ty;
        quote! {
            <#ty as ::sdk_schema::SchemaTypeDependencies>::collect_schema_types(out);
        }
    });
    let accessor_entries = fields.iter().flat_map(|field| {
        let mut entries = Vec::new();
        let value_ty = &field.ty;
        if field.getter {
            let method_name = field.schema_name.clone();
            entries.push(quote! {
                ::sdk_schema::SchemaAccessorDescriptor {
                    kind: ::sdk_schema::SchemaAccessorKind::Getter,
                    method_name: #method_name.to_string(),
                    value_field_name: "value".to_string(),
                    value_type: <#value_ty as ::sdk_schema::SchemaTypeRef>::schema_type_ref(),
                    payload_type_name: None,
                }
            });
        }
        if field.setter {
            let method_name = format!("set_{}", field.schema_name);
            let payload_type_name = format!(
                "{}Set{}Payload",
                entity_name,
                pascal_case(&field.schema_name)
            );
            entries.push(quote! {
                ::sdk_schema::SchemaAccessorDescriptor {
                    kind: ::sdk_schema::SchemaAccessorKind::Setter,
                    method_name: #method_name.to_string(),
                    value_field_name: "value".to_string(),
                    value_type: <#value_ty as ::sdk_schema::SchemaTypeRef>::schema_type_ref(),
                    payload_type_name: Some(#payload_type_name.to_string()),
                }
            });
        }
        entries
    });

    Ok(quote! {
        impl #impl_generics ::sdk_schema::SchemaEntity for #entity_ty #where_clause {
            fn schema_entity_name() -> &'static str {
                #entity_name
            }

            fn schema_type_descriptor() -> ::sdk_schema::RegistryTypeDescriptor {
                let mut descriptor = ::sdk_schema::RegistryTypeDescriptor::new(#entity_name)
                    .constructible(#constructible);
                #(#descriptor_fields)*
                descriptor
            }

            fn schema_accessors() -> Vec<::sdk_schema::SchemaAccessorDescriptor> {
                vec![#(#accessor_entries),*]
            }

            fn collect_schema_types(out: &mut Vec<::sdk_schema::RegistryTypeDescriptor>) {
                ::sdk_schema::push_schema_type_descriptor(out, Self::schema_type_descriptor());
                #(#dependency_collectors)*
            }
        }
    })
}

fn schema_name(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    let mut name = None;
    for attr in attrs {
        if let Some(value) = parse_name_value_string_attr(
            attr,
            "schema_name",
            "schema_name must be a string literal",
        )? {
            name = Some(value);
        }
    }
    Ok(name)
}

fn schema_constructible(attrs: &[Attribute]) -> syn::Result<Option<bool>> {
    let mut constructible = None;
    for attr in attrs {
        if let Some(value) = parse_name_value_bool_attr(
            attr,
            "schema_constructible",
            "schema_constructible must be a boolean literal",
        )? {
            constructible = Some(value);
        }
    }
    Ok(constructible)
}
