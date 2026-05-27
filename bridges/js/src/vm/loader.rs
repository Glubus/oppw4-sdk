use rquickjs::{
    loader::{BuiltinLoader, BuiltinResolver},
    CatchResultExt, Context, Module, Runtime,
};
use sdk_bridge::BridgeModContext;
use std::sync::{Arc, Mutex};

use crate::{module::JsModule, vm};

pub fn load(context: &BridgeModContext, modules: &[JsModule]) -> Result<vm::JsVm, String> {
    let runtime = Runtime::new().map_err(|error| format!("js runtime create failed: {error}"))?;
    install_builtin_namespace_modules(&runtime, modules);
    let js_context =
        Context::full(&runtime).map_err(|error| format!("js context create failed: {error}"))?;
    let logs = Arc::new(Mutex::new(Vec::new()));

    let handlers = js_context.with(|ctx| {
        vm::runtime::install_mod_globals(ctx.clone(), context)
            .map_err(|error| format!("js globals failed: {error}"))?;
        let handlers = vm::handlers::install(ctx.clone(), context)
            .map_err(|error| format!("js handler registry install failed: {error}"))?;
        vm::modules::install(ctx.clone(), &context.mod_id, modules, logs.clone())
            .map_err(|error| format!("js module install failed: {error}"))?;
        vm::api::install(ctx.clone()).map_err(|error| format!("js api install failed: {error}"))?;
        vm::runtime::hide_unsafe_globals(ctx.clone())
            .map_err(|error| format!("js sandbox seal failed: {error}"))?;

        let source = vm::source::read_entry_script(context)
            .map_err(|error| format!("js entry read failed: {error}"))?;
        Module::evaluate(ctx.clone(), context.entry_file.as_str(), source)
            .catch(&ctx)
            .map_err(|error| format!("js entry failed: {error}"))?
            .finish::<()>()
            .catch(&ctx)
            .map_err(|error| format!("js entry failed: {error}"))?;
        handlers.descriptors()
    })?;

    Ok(vm::JsVm::new(runtime, js_context, handlers, logs))
}

fn install_builtin_namespace_modules(runtime: &Runtime, modules: &[JsModule]) {
    let namespace_modules = vm::modules::builtin_namespace_modules(modules);
    let mut resolver = BuiltinResolver::default();
    let mut loader = BuiltinLoader::default();
    for (name, source) in namespace_modules {
        resolver.add_module(name.clone());
        loader.add_module(name, source);
    }
    runtime.set_loader(resolver, loader);
}
