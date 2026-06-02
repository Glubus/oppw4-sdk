use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemMod, ItemStruct};

mod field_attrs;
mod schema_attr;
mod schema_entity;
mod schema_module;
mod shared;

use crate::{
    schema_attr::{expand_schema_attribute, SchemaArgs},
    schema_entity::expand_schema_entity,
    schema_module::{expand_schema_module_attribute, SchemaModuleArgs},
};

#[proc_macro_derive(
    SchemaEntity,
    attributes(schema_name, schema_constructible, schema_field_name, getter, setter)
)]
pub fn derive_schema_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_schema_entity(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn schema(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as SchemaArgs);
    let item = parse_macro_input!(item as ItemStruct);
    expand_schema_attribute(attr, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn schema_module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as SchemaModuleArgs);
    let item = parse_macro_input!(item as ItemMod);
    expand_schema_module_attribute(attr, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
