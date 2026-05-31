use std::sync::Arc;

use rquickjs::{prelude::Func, Ctx};

use crate::{module::JsModule, vm::error};

pub(super) fn install(ctx: Ctx<'_>, modules: &[JsModule]) -> rquickjs::Result<()> {
    let modules = Arc::new(modules.to_vec());
    ctx.globals().set(
        "__oppw4_registry_invoke",
        Func::from(
            move |qualified_name: String, args_json: String| -> rquickjs::Result<String> {
                invoke_registry_function(&modules, &qualified_name, &args_json).map_err(|err| {
                    error::js("Registry", "Invoke", format!("{qualified_name}: {err}"))
                })
            },
        ),
    )
}

fn invoke_registry_function(
    modules: &[JsModule],
    qualified_name: &str,
    args_json: &str,
) -> Result<String, String> {
    let (namespace, import_name, function_name) = parse_qualified_function(qualified_name)?;
    let Some(module) = modules.iter().find(|module| {
        module.schema.as_ref().is_some_and(|schema| {
            schema.namespace == namespace && schema.import_name == import_name
        })
    }) else {
        return Err("module is not available".to_string());
    };
    let Some(schema) = module.schema.as_ref() else {
        return Err("module has no schema".to_string());
    };
    if !schema
        .functions
        .iter()
        .any(|function| function.name == function_name)
    {
        return Err("function is not declared by schema".to_string());
    }
    let Some(invoke) = module.invoke.as_ref() else {
        return Err("function is not bound".to_string());
    };
    invoke(function_name, args_json)
}

fn parse_qualified_function(qualified_name: &str) -> Result<(&str, &str, &str), String> {
    let mut parts = qualified_name.split('.');
    let namespace = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "missing namespace".to_string())?;
    let import_name = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "missing import name".to_string())?;
    let function_name = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "missing function name".to_string())?;
    if parts.next().is_some() {
        return Err("function name must have exactly three segments".to_string());
    }
    Ok((namespace, import_name, function_name))
}
