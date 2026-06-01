use rquickjs::Ctx;
use sdk_bridge::{
    RegistryEventDescriptor, RegistryFieldDescriptor, RegistryFunctionDescriptor,
    RegistryMethodDescriptor, RegistryModuleLoad, RegistryModuleSchema, RegistryMutationDescriptor,
    RegistryParamDescriptor, RegistryTypeDescriptor, RegistryTypeExtensionDescriptor,
    RegistryTypeRef,
};

use crate::module::JsModuleRef;

pub(super) fn install(ctx: Ctx<'_>, modules: &[JsModuleRef<'_>]) -> rquickjs::Result<()> {
    let modules_json = serde_json::Value::Array(
        modules
            .iter()
            .map(|module| {
                serde_json::json!({
                    "providerId": module.plugin_id,
                    "name": module.module_name,
                    "load": module_load_label(module.load),
                    "schema": module.schema.map(schema_json),
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
        "functions": schema.functions.iter().map(function_json).collect::<Vec<_>>(),
        "types": schema.types.iter().map(type_descriptor_json).collect::<Vec<_>>(),
        "extensions": schema.extensions.iter().map(extension_json).collect::<Vec<_>>(),
        "events": schema.events.iter().map(event_json).collect::<Vec<_>>(),
        "mutations": schema.mutations.iter().map(mutation_json).collect::<Vec<_>>(),
    })
}

fn function_json(function: &RegistryFunctionDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": function.name,
        "params": function.params.iter().map(param_json).collect::<Vec<_>>(),
        "returns": type_ref_json(&function.returns),
    })
}

fn param_json(param: &RegistryParamDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": param.name,
        "type": type_ref_json(&param.type_ref),
    })
}

fn type_descriptor_json(type_descriptor: &RegistryTypeDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": type_descriptor.name,
        "constructible": type_descriptor.constructible,
        "fields": type_descriptor.fields.iter().map(field_json).collect::<Vec<_>>(),
    })
}

fn field_json(field: &RegistryFieldDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": field.name,
        "type": type_ref_json(&field.type_ref),
    })
}

fn extension_json(extension: &RegistryTypeExtensionDescriptor) -> serde_json::Value {
    serde_json::json!({
        "targetType": extension.target_type,
        "methods": extension.methods.iter().map(method_json).collect::<Vec<_>>(),
    })
}

fn method_json(method: &RegistryMethodDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": method.name,
        "function": method.function,
        "returns": type_ref_json(&method.returns),
    })
}

fn event_json(event: &RegistryEventDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": event.name,
        "key": event.key,
        "payload": type_ref_json(&event.payload),
    })
}

fn mutation_json(mutation: &RegistryMutationDescriptor) -> serde_json::Value {
    serde_json::json!({
        "name": mutation.name,
        "key": mutation.key,
        "payload": type_ref_json(&mutation.payload),
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
