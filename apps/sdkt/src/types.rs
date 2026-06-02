mod names;
mod render;
mod runtime;

#[cfg(test)]
mod tests;

use std::{fs, path::Path};

use sdk_runtime_schema::runtime_schemas as generated_runtime_schemas;
use sdk_schema::RegistryModuleSchema;

const TYPES_DIR: &str = ".sdkt/types/oppw4";

pub(crate) fn install_types(root: &Path) -> Result<(), String> {
    let types_dir = root.join(TYPES_DIR);
    fs::create_dir_all(&types_dir).map_err(|error| format!("{}: {error}", types_dir.display()))?;

    let files = render_typescript_files()?;
    for (name, contents) in &files {
        let path = types_dir.join(name);
        fs::write(&path, contents).map_err(|error| format!("{}: {error}", path.display()))?;
    }

    let legacy_root_types = root.join("sdkt-types.d.ts");
    if legacy_root_types.exists() {
        fs::remove_file(&legacy_root_types)
            .map_err(|error| format!("{}: {error}", legacy_root_types.display()))?;
    }

    println!(
        "installed TypeScript declarations in {}",
        types_dir.display()
    );
    Ok(())
}

fn render_typescript_files() -> Result<Vec<(String, String)>, String> {
    let schemas = runtime_schemas();
    let mut files = Vec::new();
    let mut refs = vec![
        "globals.d.ts".to_string(),
        "sdk.d.ts".to_string(),
        "character.d.ts".to_string(),
    ];

    files.push((
        "globals.d.ts".to_string(),
        runtime::render_global_declarations(),
    ));
    files.push((
        "sdk.d.ts".to_string(),
        render::render_sdk_default_module(&schemas),
    ));
    files.push((
        "character.d.ts".to_string(),
        runtime::render_character_module(),
    ));

    for schema in &schemas {
        let name = format!("{}.d.ts", schema.import_name);
        refs.push(name.clone());
        files.push((name, render::render_schema_module(schema)));
    }

    files.push(("index.d.ts".to_string(), render_index_file(&refs)));
    Ok(files)
}

fn render_index_file(files: &[String]) -> String {
    let mut output = String::new();
    for file in files {
        output.push_str(&format!("/// <reference path=\"./{file}\" />\n"));
    }
    output.push_str("export {};\n");
    output
}

fn runtime_schemas() -> Vec<RegistryModuleSchema> {
    generated_runtime_schemas()
}
