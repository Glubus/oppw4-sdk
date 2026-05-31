use rquickjs::{
    loader::{BuiltinLoader, BuiltinResolver},
    CatchResultExt, Context, Module, Runtime,
};
use sdk_bridge::{BridgeAnalysisReport, BridgeModContext};
use std::sync::{Arc, Mutex};

use crate::{
    module::JsModule,
    vm::{self, error::StringContext},
};

pub fn load(context: &BridgeModContext, modules: &[JsModule]) -> Result<vm::JsVm, String> {
    let runtime = Runtime::new().context("js runtime create failed")?;
    install_builtin_namespace_modules(&runtime, modules);
    let js_context = Context::full(&runtime).context("js context create failed")?;
    let logs = Arc::new(Mutex::new(Vec::new()));

    let source = vm::source::read_entry_script(context).context("js entry read failed")?;
    let analysis = analyze_source(context, &source);

    let handlers = js_context.with(|ctx| {
        vm::runtime::install_mod_globals(ctx.clone(), context).context("js globals failed")?;
        let handlers = vm::handlers::install(ctx.clone(), context)
            .context("js handler registry install failed")?;
        vm::modules::install(ctx.clone(), &context.mod_id, modules, logs.clone())
            .context("js module install failed")?;
        vm::api::install(ctx.clone()).context("js api install failed")?;
        vm::runtime::hide_unsafe_globals(ctx.clone()).context("js sandbox seal failed")?;

        Module::evaluate(ctx.clone(), context.entry_file.as_str(), source)
            .catch(&ctx)
            .context("js entry failed")?
            .finish::<()>()
            .catch(&ctx)
            .context("js entry failed")?;
        handlers.descriptors()
    })?;

    Ok(vm::JsVm::new(runtime, js_context, handlers, logs, analysis))
}

fn analyze_source(context: &BridgeModContext, source: &str) -> BridgeAnalysisReport {
    let report = sdk_js_analyzer::analyze(source, &context.modules);
    BridgeAnalysisReport {
        effects: report.effects,
        warnings: report.warnings,
    }
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
