use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = generate_runtime_projection();
    fs::write(out_dir.join("generated_runtime_projection.js"), generated)
        .expect("write generated_runtime_projection.js");
}

fn generate_runtime_projection() -> String {
    let schemas = sdk_runtime_schema::runtime_schemas();

    let mut property_modules = Vec::new();
    let mut rank_calc_events = Vec::new();

    for schema in schemas {
        let qualified_module = format!("{}.{}", schema.namespace, schema.import_name);
        if qualified_module == "sdk.snapshot" {
            let properties = schema
                .functions
                .iter()
                .filter(|function| function.params.is_empty())
                .map(|function| function.name.clone())
                .collect::<Vec<_>>();
            property_modules.push((qualified_module.clone(), properties));
        }
        if qualified_module == "sdk.rank" {
            for event in &schema.events {
                match event.name.as_str() {
                    "calc_count" => rank_calc_events.push((
                        qualified_module.clone(),
                        event.name.clone(),
                        "count",
                    )),
                    "calc_time" => rank_calc_events.push((
                        qualified_module.clone(),
                        event.name.clone(),
                        "time",
                    )),
                    _ => {}
                }
            }
        }
    }

    let property_entries = property_modules
        .into_iter()
        .map(|(module, properties)| {
            let props = properties
                .into_iter()
                .map(|name| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("        {module:?}: [{props}],")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let calc_entries = rank_calc_events
        .into_iter()
        .map(|(module, event, kind)| {
            format!("        {:?}: {kind:?},", format!("{module}.{event}"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
    function generatedPropertyModules() {{
        return Object.freeze({{
{property_entries}
        }});
    }}

    function generatedRankCalcEvents() {{
        return Object.freeze({{
{calc_entries}
        }});
    }}

    function generatedCreateSchemaModule(registryModuleList, currentMod, schema) {{
        const moduleKey = `${{String(schema.namespace)}}.${{String(schema.importName)}}`;
        const properties = generatedPropertyModules()[moduleKey];
        if (!Array.isArray(properties)) {{
            return null;
        }}
        const moduleObject = Object.create(null);
        for (const name of properties) {{
            Object.defineProperty(moduleObject, name, {{
                enumerable: true,
                get() {{
                    const result = invokeRegistry(currentMod, `${{moduleKey}}.${{name}}`, []);
                    const fn = (schema.functions || []).find((candidate) => String(candidate.name || "") === name);
                    return wrapRegistryValue(
                        registryModuleList,
                        currentMod,
                        fn ? fn.returns : null,
                        result,
                        schema
                    );
                }},
            }});
        }}
        defineSchema(moduleObject, schema);
        return moduleObject;
    }}

    function generatedCallTypedEventCallback(registryModuleList, schema, event, callback, ctx) {{
        const eventKey = `${{String(schema.namespace)}}.${{String(schema.importName)}}.${{String(event.name)}}`;
        const rankCalcKind = generatedRankCalcEvents()[eventKey];
        if (rankCalcKind) {{
            return callback(freeze(projectRankCalcContext(ctx, rankCalcKind)));
        }}
        return undefined;
    }}
"#
    )
}
