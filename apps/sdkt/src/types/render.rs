use sdk_schema::{
    RegistryFunctionDescriptor, RegistryMethodDescriptor, RegistryModuleSchema,
    RegistryMutationDescriptor, RegistryTypeDescriptor, RegistryTypeRef,
};

use super::names::{pascal_case, ts_type_name};
use super::runtime;

pub(super) fn render_sdk_default_module(schemas: &[RegistryModuleSchema]) -> String {
    let mut export_names = schemas
        .iter()
        .map(|schema| schema.import_name.clone())
        .collect::<Vec<_>>();
    export_names.push("character".to_string());
    export_names.sort_unstable();
    export_names.dedup();

    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export type JsonValue = unknown;\n");
    output.push_str("  export type JsonObject = Record<string, unknown>;\n\n");
    output.push_str("  const sdk: {\n");
    for export_name in &export_names {
        let type_name = if export_name == "character" {
            "CharacterNamespace".to_string()
        } else {
            pascal_case(export_name)
        };
        output.push_str(&format!("    {export_name}: {type_name};\n"));
    }
    output.push_str("  };\n");
    output.push_str("  export default sdk;\n");
    output.push_str("}\n");
    output
}

pub(super) fn render_schema_module(schema: &RegistryModuleSchema) -> String {
    if let Some(rendered) = runtime::render_known_runtime_module(&schema.import_name) {
        return rendered;
    }

    let mut output = String::new();
    let module_type = pascal_case(&schema.import_name);
    output.push_str("declare module \"sdk\" {\n");

    for type_descriptor in &schema.types {
        output.push_str(&render_type_descriptor(schema, type_descriptor, 2));
        output.push('\n');
    }

    output.push_str(&format!("  export interface {module_type} {{\n"));
    for function in &schema.functions {
        output.push_str(&render_function_descriptor(function, 4));
        output.push('\n');
    }
    for event in &schema.events {
        output.push_str(&render_event_descriptor(event, 4));
        output.push('\n');
    }
    output.push_str("  }\n\n");
    output.push_str(&format!(
        "  export const {}: {module_type};\n",
        schema.import_name
    ));
    output.push_str("}\n");
    output
}

fn render_type_descriptor(
    schema: &RegistryModuleSchema,
    type_descriptor: &RegistryTypeDescriptor,
    indent: usize,
) -> String {
    let mut output = String::new();
    let padding = " ".repeat(indent);
    output.push_str(&format!(
        "{padding}export interface {} {{\n",
        type_descriptor.name
    ));
    for field in &type_descriptor.fields {
        output.push_str(&format!(
            "{}{}: {};\n",
            " ".repeat(indent + 2),
            field.name,
            render_type_ref(&field.type_ref)
        ));
    }
    for method in extension_methods_for_type(schema, &type_descriptor.name) {
        output.push_str(&render_extension_method_descriptor(
            schema,
            method,
            indent + 2,
        ));
        output.push('\n');
    }
    output.push_str(&format!("{padding}}}\n"));
    output
}

fn render_function_descriptor(function: &RegistryFunctionDescriptor, indent: usize) -> String {
    let params = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, render_type_ref(&param.type_ref)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}{}({}): {};",
        " ".repeat(indent),
        function.name,
        params,
        render_type_ref(&function.returns)
    )
}

fn render_event_descriptor(event: &sdk_schema::RegistryEventDescriptor, indent: usize) -> String {
    format!(
        "{}on_{}(callback: (ctx: Oppw4EventContext<{}>) => void): string;",
        " ".repeat(indent),
        event.name,
        render_type_ref(&event.payload)
    )
}

fn extension_methods_for_type<'a>(
    schema: &'a RegistryModuleSchema,
    type_name: &str,
) -> Vec<&'a RegistryMethodDescriptor> {
    let qualified_name = format!("{}.{}", schema.namespace, type_name);
    schema
        .extensions
        .iter()
        .filter(|extension| {
            extension.target_type == type_name || extension.target_type == qualified_name
        })
        .flat_map(|extension| extension.methods.iter())
        .collect()
}

fn render_extension_method_descriptor(
    schema: &RegistryModuleSchema,
    method: &RegistryMethodDescriptor,
    indent: usize,
) -> String {
    let params = extension_method_params(schema, method)
        .into_iter()
        .map(|(name, type_ref)| format!("{name}: {}", render_type_ref(&type_ref)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}{}({}): {};",
        " ".repeat(indent),
        method.name,
        params,
        render_type_ref(&method.returns)
    )
}

fn extension_method_params(
    schema: &RegistryModuleSchema,
    method: &RegistryMethodDescriptor,
) -> Vec<(String, RegistryTypeRef)> {
    if let Some(function_name) = &method.function {
        if let Some(function) = schema.functions.iter().find(|f| &f.name == function_name) {
            return function
                .params
                .iter()
                .skip(1)
                .map(|param| (param.name.clone(), param.type_ref.clone()))
                .collect();
        }
    }
    if let Some(mutation_name) = &method.mutation {
        if let Some(mutation) = schema.mutations.iter().find(|m| &m.name == mutation_name) {
            return mutation_params(schema, mutation);
        }
    }
    Vec::new()
}

fn mutation_params(
    schema: &RegistryModuleSchema,
    mutation: &RegistryMutationDescriptor,
) -> Vec<(String, RegistryTypeRef)> {
    match &mutation.payload {
        RegistryTypeRef::Named { name } => {
            let descriptor = schema.types.iter().find(|ty| {
                ty.name == *name || format!("{}.{}", schema.namespace, ty.name) == *name
            });
            if let Some(descriptor) = descriptor {
                return descriptor
                    .fields
                    .iter()
                    .filter(|field| field.name != "target")
                    .map(|field| (field.name.clone(), field.type_ref.clone()))
                    .collect();
            }
            vec![("payload".to_string(), mutation.payload.clone())]
        }
        _ => vec![("payload".to_string(), mutation.payload.clone())],
    }
}

fn render_type_ref(type_ref: &RegistryTypeRef) -> String {
    match type_ref {
        RegistryTypeRef::Named { name } => ts_type_name(name.split('.').last().unwrap_or(name)),
        RegistryTypeRef::Optional { inner } => {
            format!("{} | null | undefined", render_type_ref(inner))
        }
        RegistryTypeRef::Array { inner } => format!("ReadonlyArray<{}>", render_type_ref(inner)),
        RegistryTypeRef::Void => "void".to_string(),
        RegistryTypeRef::Bool => "boolean".to_string(),
        RegistryTypeRef::I64 | RegistryTypeRef::F64 => "number".to_string(),
        RegistryTypeRef::String => "string".to_string(),
        RegistryTypeRef::Json => "JsonValue".to_string(),
    }
}
