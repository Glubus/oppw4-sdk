use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Fields, ItemStruct, LitBool, LitStr, Result, Token,
};

use crate::{field_attrs::parse_schema_macro_fields, schema_entity::expand_entity_impl};

pub(crate) fn expand_schema_attribute(
    attr: SchemaArgs,
    mut item: ItemStruct,
) -> syn::Result<TokenStream> {
    let ident = item.ident.clone();
    let generics = item.generics.clone();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let entity_name = attr.name.unwrap_or_else(|| ident.to_string());
    let constructible = attr.constructible.unwrap_or(false);

    let Fields::Named(fields) = &mut item.fields else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "schema attribute requires a struct with named fields",
        ));
    };
    let schema_fields = parse_schema_macro_fields(fields)?;
    let entity_impl = expand_entity_impl(
        quote!(#ident #type_generics),
        quote!(#impl_generics),
        where_clause,
        entity_name,
        constructible,
        schema_fields,
    )?;

    Ok(quote! {
        #item
        #entity_impl
    })
}

pub(crate) struct SchemaArgs {
    pub(crate) name: Option<String>,
    pub(crate) constructible: Option<bool>,
}

impl Parse for SchemaArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut args = Self {
            name: None,
            constructible: None,
        };

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if ident == "name" {
                let lit: LitStr = input.parse()?;
                args.name = Some(lit.value());
            } else if ident == "constructible" {
                let lit: LitBool = input.parse()?;
                args.constructible = Some(lit.value());
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "unsupported schema argument",
                ));
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(args)
    }
}
