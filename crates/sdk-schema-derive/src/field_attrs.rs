use syn::{Expr, Field, Fields, FieldsNamed, Type};

use crate::shared::{parse_expr_string, parse_name_value_string_attr};

#[derive(Clone)]
pub(crate) struct SchemaField {
    pub(crate) schema_name: String,
    pub(crate) ty: Type,
    pub(crate) getter: bool,
    pub(crate) setter: bool,
}

pub(crate) fn parse_owned_fields(fields: Fields) -> syn::Result<Vec<SchemaField>> {
    let Fields::Named(fields) = fields else {
        return Err(syn::Error::new_spanned(
            fields,
            "SchemaEntity requires a struct with named fields",
        ));
    };

    fields.named.into_iter().map(parse_field_attrs).collect()
}

pub(crate) fn parse_schema_macro_fields(fields: &mut FieldsNamed) -> syn::Result<Vec<SchemaField>> {
    fields
        .named
        .iter_mut()
        .map(|field| {
            let mut schema_name = field.ident.as_ref().expect("named field").to_string();
            let mut getter = false;
            let mut setter = false;
            let mut retained = Vec::with_capacity(field.attrs.len());

            for attr in field.attrs.drain(..) {
                if attr.path().is_ident("schema") {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("name") {
                            let value = meta.value()?;
                            let expr: Expr = value.parse()?;
                            schema_name =
                                parse_expr_string(&expr, "schema name must be a string literal")?;
                        }
                        Ok(())
                    })?;
                } else if attr.path().is_ident("getter") {
                    getter = true;
                } else if attr.path().is_ident("setter") {
                    setter = true;
                } else {
                    retained.push(attr);
                }
            }

            field.attrs = retained;
            Ok(SchemaField {
                schema_name,
                ty: field.ty.clone(),
                getter,
                setter,
            })
        })
        .collect()
}

fn parse_field_attrs(mut field: Field) -> syn::Result<SchemaField> {
    let mut schema_name = field.ident.as_ref().expect("named field").to_string();
    let mut getter = false;
    let mut setter = false;
    let mut retained = Vec::with_capacity(field.attrs.len());

    for attr in field.attrs.drain(..) {
        if let Some(value) = parse_name_value_string_attr(
            &attr,
            "schema_field_name",
            "schema_field_name must be a string literal",
        )? {
            schema_name = value;
        } else if attr.path().is_ident("getter") {
            getter = true;
        } else if attr.path().is_ident("setter") {
            setter = true;
        } else {
            retained.push(attr);
        }
    }

    field.attrs = retained;
    Ok(SchemaField {
        schema_name,
        ty: field.ty,
        getter,
        setter,
    })
}
