mod invoke;
mod metadata;
mod namespace;
mod register;
mod trace;

use std::sync::mpsc::Sender;

use rquickjs::Ctx;
use sdk_bridge::ModId;

use crate::module::JsModuleRef;

pub(super) fn install(
    ctx: Ctx<'_>,
    mod_id: &ModId,
    modules: &[JsModuleRef<'_>],
    logs: Sender<String>,
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

pub(super) fn builtin_namespace_modules(modules: &[JsModuleRef<'_>]) -> Vec<(String, String)> {
    namespace::builtin_namespace_modules(modules)
}

fn log_contract_errors(mod_id: &ModId, modules: &[JsModuleRef<'_>], logs: &Sender<String>) {
    for module in modules {
        let Some(schema) = module.schema else {
            continue;
        };
        if let Err(error) = schema.validate_contract() {
            let _ = logs.send(format!(
                "registry contract warning mod={} module={} error={error}",
                mod_id.as_str(),
                module.module_name
            ));
        }
    }
}
