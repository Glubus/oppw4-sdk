use syn::{Attribute, Expr, ExprLit, Lit};

pub(crate) fn parse_name_value_string_attr(
    attr: &Attribute,
    expected_attr: &str,
    message: &str,
) -> syn::Result<Option<String>> {
    if !attr.path().is_ident(expected_attr) {
        return Ok(None);
    }

    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = &attr.meta.require_name_value()?.value
    else {
        return Err(syn::Error::new_spanned(&attr.meta, message));
    };

    Ok(Some(lit.value()))
}

pub(crate) fn parse_name_value_bool_attr(
    attr: &Attribute,
    expected_attr: &str,
    message: &str,
) -> syn::Result<Option<bool>> {
    if !attr.path().is_ident(expected_attr) {
        return Ok(None);
    }

    let Expr::Lit(ExprLit {
        lit: Lit::Bool(lit),
        ..
    }) = &attr.meta.require_name_value()?.value
    else {
        return Err(syn::Error::new_spanned(&attr.meta, message));
    };

    Ok(Some(lit.value()))
}

pub(crate) fn parse_expr_string(expr: &Expr, message: &str) -> syn::Result<String> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = expr
    else {
        return Err(syn::Error::new_spanned(expr, message));
    };

    Ok(lit.value())
}

pub(crate) fn pascal_case(value: &str) -> String {
    let mut output = String::new();
    for part in value.split('_').filter(|part| !part.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            output.extend(chars);
        }
    }
    output
}
