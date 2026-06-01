use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use rquickjs::{prelude::Func, Ctx};

use crate::{module::JsModuleInvoke, module::JsModuleRef, vm::error};

pub(super) fn install(ctx: Ctx<'_>, modules: &[JsModuleRef<'_>]) -> rquickjs::Result<()> {
    let table = Arc::new(RegistryInvokeTable::new(modules));
    ctx.globals().set(
        "__oppw4_registry_invoke",
        Func::from(
            move |qualified_name: String, args_json: String| -> rquickjs::Result<String> {
                table.invoke(&qualified_name, &args_json).map_err(|err| {
                    error::js("Registry", "Invoke", format!("{qualified_name}: {err}"))
                })
            },
        ),
    )
}

struct RegistryInvokeTable {
    modules: HashSet<String>,
    functions: HashSet<String>,
    bound: HashMap<String, RegistryFunctionBinding>,
}

struct RegistryFunctionBinding {
    function_name: String,
    invoke: JsModuleInvoke,
}

impl RegistryInvokeTable {
    fn new(modules: &[JsModuleRef<'_>]) -> Self {
        let mut table = Self {
            modules: HashSet::new(),
            functions: HashSet::new(),
            bound: HashMap::new(),
        };
        for module in modules {
            let Some(schema) = module.schema else {
                continue;
            };
            let module_key = module_key(&schema.namespace, &schema.import_name);
            table.modules.insert(module_key);
            for function in &schema.functions {
                let qualified_name =
                    qualified_function_name(&schema.namespace, &schema.import_name, &function.name);
                table.functions.insert(qualified_name.clone());
                if let Some(invoke) = module.invoke {
                    table.bound.insert(
                        qualified_name,
                        RegistryFunctionBinding {
                            function_name: function.name.clone(),
                            invoke: Arc::clone(invoke),
                        },
                    );
                }
            }
        }
        table
    }

    fn invoke(&self, qualified_name: &str, args_json: &str) -> Result<String, String> {
        let (namespace, import_name, _function_name) = parse_qualified_function(qualified_name)?;
        if !self.modules.contains(&module_key(namespace, import_name)) {
            return Err("module is not available".to_string());
        }
        if !self.functions.contains(qualified_name) {
            return Err("function is not declared by schema".to_string());
        }
        let Some(binding) = self.bound.get(qualified_name) else {
            return Err("function is not bound".to_string());
        };
        (binding.invoke)(&binding.function_name, args_json)
    }
}

fn module_key(namespace: &str, import_name: &str) -> String {
    format!("{namespace}.{import_name}")
}

fn qualified_function_name(namespace: &str, import_name: &str, function_name: &str) -> String {
    format!("{namespace}.{import_name}.{function_name}")
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
