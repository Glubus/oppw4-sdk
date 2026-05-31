mod invoke;
mod metadata;
mod namespace;
mod register;
mod trace;

use std::sync::{Arc, Mutex};

use rquickjs::Ctx;
use sdk_bridge::ModId;

use crate::module::JsModule;

pub(super) fn install(
    ctx: Ctx<'_>,
    mod_id: &ModId,
    modules: &[JsModule],
    logs: Arc<Mutex<Vec<String>>>,
) -> rquickjs::Result<()> {
    log_contract_errors(mod_id, modules, &logs);
    trace::install(ctx.clone(), mod_id, logs)?;
    metadata::install(ctx.clone(), modules)?;
    invoke::install(ctx.clone(), modules)?;
    for module in modules {
        register::register_plugin_module(ctx.clone(), module)?;
    }
    Ok(())
}

pub(super) fn builtin_namespace_modules(modules: &[JsModule]) -> Vec<(String, String)> {
    namespace::builtin_namespace_modules(modules)
}

fn log_contract_errors(mod_id: &ModId, modules: &[JsModule], logs: &Arc<Mutex<Vec<String>>>) {
    let Ok(mut logs) = logs.lock() else {
        return;
    };
    for module in modules {
        let Some(schema) = &module.schema else {
            continue;
        };
        if let Err(error) = schema.validate_contract() {
            logs.push(format!(
                "registry contract warning mod={} module={} error={error}",
                mod_id.as_str(),
                module.module_name
            ));
        }
    }
}
