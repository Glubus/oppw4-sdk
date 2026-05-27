use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::{ModId, RegistryModuleLoad, RegistryModuleSchema, RegistryTypeRef};

use crate::module::JsModule;

pub(super) fn install(
    ctx: Ctx<'_>,
    mod_id: &ModId,
    modules: &[JsModule],
    logs: Arc<Mutex<Vec<String>>>,
) -> rquickjs::Result<()> {
    install_trace(ctx.clone(), mod_id, logs)?;
    install_registry_metadata(ctx.clone(), modules)?;
    install_registry_invoker(ctx.clone(), modules)?;
    for module in modules {
        register_plugin_module(ctx.clone(), module)?;
    }
    Ok(())
}

fn install_registry_invoker(ctx: Ctx<'_>, modules: &[JsModule]) -> rquickjs::Result<()> {
    let modules = Arc::new(modules.to_vec());
    ctx.globals().set(
        "__oppw4_registry_invoke",
        Func::from(
            move |qualified_name: String, args_json: String| -> rquickjs::Result<String> {
                invoke_registry_function(&modules, &qualified_name, &args_json).map_err(|error| {
                    rquickjs::Error::new_from_js_message(
                        "Registry",
                        "Invoke",
                        format!("{qualified_name}: {error}"),
                    )
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

pub(super) fn builtin_namespace_modules(modules: &[JsModule]) -> Vec<(String, String)> {
    let mut namespaces = modules
        .iter()
        .filter_map(|module| module.schema.as_ref())
        .map(|schema| schema.namespace.as_str())
        .filter(|namespace| is_js_identifier(namespace))
        .collect::<Vec<_>>();
    namespaces.sort_unstable();
    namespaces.dedup();

    namespaces
        .into_iter()
        .map(|namespace| {
            (
                namespace.to_string(),
                namespace_module_source(namespace, modules),
            )
        })
        .collect()
}

fn namespace_module_source(namespace: &str, modules: &[JsModule]) -> String {
    let imports = modules
        .iter()
        .filter_map(|module| module.schema.as_ref())
        .filter(|schema| schema.namespace == namespace && is_js_identifier(&schema.import_name))
        .map(|schema| schema.import_name.as_str())
        .collect::<Vec<_>>();

    let mut source = String::new();
    for import_name in &imports {
        source.push_str("export const ");
        source.push_str(import_name);
        source.push_str(" = globalThis[");
        source.push_str(&serde_json::to_string(namespace).expect("json string"));
        source.push_str("][");
        source.push_str(&serde_json::to_string(import_name).expect("json string"));
        source.push_str("];\n");
    }
    source.push_str("export default Object.freeze({");
    for (index, import_name) in imports.iter().enumerate() {
        if index > 0 {
            source.push(',');
        }
        source.push_str(import_name);
    }
    source.push_str("});\n");
    source
}

fn is_js_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first == '$' || first.is_ascii_alphabetic())
        && chars.all(|char| char == '_' || char == '$' || char.is_ascii_alphanumeric())
}

pub(super) fn install_trace(
    ctx: Ctx<'_>,
    mod_id: &ModId,
    logs: Arc<Mutex<Vec<String>>>,
) -> rquickjs::Result<()> {
    let mod_id = mod_id.as_str().to_string();
    ctx.globals().set(
        "__oppw4_trace",
        Func::from(move |message: String| {
            if let Ok(mut logs) = logs.lock() {
                logs.push(format!("js trace mod={mod_id} {message}"));
            }
        }),
    )
}

fn install_registry_metadata(ctx: Ctx<'_>, modules: &[JsModule]) -> rquickjs::Result<()> {
    let modules_json = serde_json::Value::Array(
        modules
            .iter()
            .map(|module| {
                serde_json::json!({
                    "providerId": module.plugin_id,
                    "name": module.module_name,
                    "load": module_load_label(module.load),
                    "schema": module.schema.as_ref().map(schema_json),
                })
            })
            .collect(),
    )
    .to_string();
    ctx.globals()
        .set("__oppw4_registry_modules_json", modules_json)
}

fn schema_json(schema: &RegistryModuleSchema) -> serde_json::Value {
    serde_json::json!({
        "namespace": schema.namespace,
        "importName": schema.import_name,
        "constructible": schema.constructible,
        "functions": schema.functions.iter().map(|function| {
            serde_json::json!({
                "name": function.name,
                "params": function.params.iter().map(|param| {
                    serde_json::json!({
                        "name": param.name,
                        "type": type_ref_json(&param.type_ref),
                    })
                }).collect::<Vec<_>>(),
                "returns": type_ref_json(&function.returns),
            })
        }).collect::<Vec<_>>(),
        "types": schema.types.iter().map(|type_descriptor| {
            serde_json::json!({
                "name": type_descriptor.name,
                "constructible": type_descriptor.constructible,
                "fields": type_descriptor.fields.iter().map(|field| {
                    serde_json::json!({
                        "name": field.name,
                        "type": type_ref_json(&field.type_ref),
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    })
}

fn type_ref_json(type_ref: &RegistryTypeRef) -> serde_json::Value {
    match type_ref {
        RegistryTypeRef::Named { name } => serde_json::json!({ "kind": "named", "name": name }),
        RegistryTypeRef::Optional { inner } => {
            serde_json::json!({ "kind": "optional", "inner": type_ref_json(inner) })
        }
        RegistryTypeRef::Array { inner } => {
            serde_json::json!({ "kind": "array", "inner": type_ref_json(inner) })
        }
        RegistryTypeRef::Void => serde_json::json!({ "kind": "void" }),
        RegistryTypeRef::Bool => serde_json::json!({ "kind": "bool" }),
        RegistryTypeRef::I64 => serde_json::json!({ "kind": "i64" }),
        RegistryTypeRef::F64 => serde_json::json!({ "kind": "f64" }),
        RegistryTypeRef::String => serde_json::json!({ "kind": "string" }),
        RegistryTypeRef::Json => serde_json::json!({ "kind": "json" }),
    }
}

fn module_load_label(load: RegistryModuleLoad) -> &'static str {
    match load {
        RegistryModuleLoad::WhenPluginRequested => "when_plugin_requested",
        RegistryModuleLoad::Always => "always",
    }
}

fn register_plugin_module(ctx: Ctx<'_>, module: &JsModule) -> rquickjs::Result<()> {
    let result = unsafe {
        (module.register)(
            module.context as *mut c_void,
            (&ctx as *const Ctx<'_>).cast_mut().cast(),
        )
    };
    if result != 0 {
        return Err(rquickjs::Error::new_from_js_message(
            "Rust",
            "JsModule",
            format!(
                "js module register failed plugin={} module={} result={result}",
                module.plugin_id, module.module_name
            ),
        ));
    }
    Ok(())
}
