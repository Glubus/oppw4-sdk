use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    ItemMod, LitStr, Result, Token, Type,
};

pub(crate) fn expand_schema_module_attribute(
    attr: SchemaModuleArgs,
    mut item: ItemMod,
) -> syn::Result<TokenStream> {
    let Some((_, items)) = &mut item.content else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "schema_module requires an inline module",
        ));
    };

    let namespace = attr.namespace;
    let import_name = attr.import_name;
    let entity_ty = attr.entity;

    let module_fn = quote! {
        pub fn schema_module() -> ::sdk_schema::RegistryModuleSchema {
            let mut type_descriptors = Vec::new();
            <#entity_ty as ::sdk_schema::SchemaTypeDependencies>::collect_schema_types(&mut type_descriptors);

            let mut schema = ::sdk_schema::RegistryModuleSchema::new(#namespace, #import_name);
            for type_descriptor in type_descriptors {
                ::sdk_schema::push_schema_type_descriptor(&mut schema.types, type_descriptor);
            }

            let target_type = format!(
                "{}.{}",
                #namespace,
                <#entity_ty as ::sdk_schema::SchemaEntity>::schema_entity_name(),
            );

            for accessor in <#entity_ty as ::sdk_schema::SchemaEntity>::schema_accessors() {
                match accessor.kind {
                    ::sdk_schema::SchemaAccessorKind::Getter => {
                        let function_name = accessor.method_name.clone();
                        schema = schema.function(
                            ::sdk_schema::RegistryFunctionDescriptor::new(
                                function_name.clone(),
                                accessor.value_type.clone(),
                            )
                            .param(
                                "target",
                                <#entity_ty as ::sdk_schema::SchemaTypeRef>::schema_type_ref(),
                            ),
                        );
                        let method = ::sdk_schema::RegistryMethodDescriptor::new(
                            function_name,
                            accessor.method_name.clone(),
                            accessor.value_type,
                        );
                        if let Some(extension) = schema
                            .extensions
                            .iter_mut()
                            .find(|extension| extension.target_type == target_type)
                        {
                            extension.methods.push(method);
                        } else {
                            schema = schema.extension(
                                ::sdk_schema::RegistryTypeExtensionDescriptor::new(
                                    target_type.clone(),
                                )
                                .method(method),
                            );
                        }
                    }
                    ::sdk_schema::SchemaAccessorKind::Setter => {
                        let payload_type_name =
                            accessor.payload_type_name.expect("setter payload type");
                        schema = schema.type_descriptor(
                            ::sdk_schema::RegistryTypeDescriptor::new(payload_type_name.clone())
                                .field(
                                    "target",
                                    <#entity_ty as ::sdk_schema::SchemaTypeRef>::schema_type_ref(),
                                )
                                .field(accessor.value_field_name.clone(), accessor.value_type.clone()),
                        );
                        schema = schema.mutation(::sdk_schema::RegistryMutationDescriptor::new(
                            accessor.method_name.clone(),
                            format!("{}.{}.{}", #namespace, #import_name, accessor.method_name),
                            ::sdk_schema::RegistryTypeRef::Named {
                                name: payload_type_name,
                            },
                        ));
                        let method = ::sdk_schema::RegistryMethodDescriptor::mutation(
                            accessor.method_name.clone(),
                            accessor.method_name,
                            ::sdk_schema::RegistryTypeRef::Void,
                        );
                        if let Some(extension) = schema
                            .extensions
                            .iter_mut()
                            .find(|extension| extension.target_type == target_type)
                        {
                            extension.methods.push(method);
                        } else {
                            schema = schema.extension(
                                ::sdk_schema::RegistryTypeExtensionDescriptor::new(
                                    target_type.clone(),
                                )
                                .method(method),
                            );
                        }
                    }
                }
            }

            schema
        }
    };

    items.push(syn::parse_quote!(#module_fn));
    Ok(quote! { #item })
}

pub(crate) struct SchemaModuleArgs {
    pub(crate) namespace: String,
    pub(crate) import_name: String,
    pub(crate) entity: Type,
}

impl Parse for SchemaModuleArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut namespace = None;
        let mut import_name = None;
        let mut entity = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if ident == "namespace" {
                let lit: LitStr = input.parse()?;
                namespace = Some(lit.value());
            } else if ident == "import_name" {
                let lit: LitStr = input.parse()?;
                import_name = Some(lit.value());
            } else if ident == "entity" {
                entity = Some(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "unsupported schema_module argument",
                ));
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            namespace: namespace.ok_or_else(|| input.error("missing namespace"))?,
            import_name: import_name.ok_or_else(|| input.error("missing import_name"))?,
            entity: entity.ok_or_else(|| input.error("missing entity"))?,
        })
    }
}
