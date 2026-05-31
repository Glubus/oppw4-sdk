use crate::module::JsModule;

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
    let mut imports = modules
        .iter()
        .filter_map(|module| module.schema.as_ref())
        .filter(|schema| schema.namespace == namespace && is_js_identifier(&schema.import_name))
        .map(|schema| schema.import_name.as_str())
        .collect::<Vec<_>>();
    imports.sort_unstable();
    imports.dedup();

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
