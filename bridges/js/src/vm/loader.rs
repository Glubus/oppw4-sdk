use rquickjs::{
    loader::{Loader, Resolver},
    module::Declared,
    CatchResultExt, Context, Ctx, Error, Module, Runtime,
};
use sdk_bridge::{BridgeAnalysisReport, BridgeModContext};
use std::sync::mpsc;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    module::JsModuleRef,
    vm::{self, error::StringContext},
};

pub fn load(context: &BridgeModContext, modules: &[JsModuleRef<'_>]) -> Result<vm::JsVm, String> {
    let runtime = Runtime::new().context("js runtime create failed")?;
    install_module_loader(&runtime, context, modules);
    let js_context = Context::full(&runtime).context("js context create failed")?;
    let (logs, log_receiver) = mpsc::channel();

    let source = vm::source::read_entry_script(context).context("js entry read failed")?;
    let analysis = analyze_source(context, &source);
    let source = vm::source::transpile_script(&context.entry_file, &source)
        .map_err(|error| format!("js entry transpile failed: {error}"))?;

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

    Ok(vm::JsVm::new(
        runtime,
        js_context,
        handlers,
        log_receiver,
        analysis,
    ))
}

fn analyze_source(context: &BridgeModContext, source: &str) -> BridgeAnalysisReport {
    let report = sdk_js_analyzer::analyze(source, &context.modules);
    BridgeAnalysisReport {
        effects: report.effects,
        warnings: report.warnings,
    }
}

fn install_module_loader(
    runtime: &Runtime,
    context: &BridgeModContext,
    modules: &[JsModuleRef<'_>],
) {
    let namespace_modules = vm::modules::builtin_namespace_modules(modules)
        .into_iter()
        .collect::<HashMap<_, _>>();
    let resolver = ModResolver::new(namespace_modules.keys().cloned().collect());
    let loader = ModLoader::new(context.clone(), namespace_modules);
    runtime.set_loader(resolver, loader);
}

#[derive(Debug)]
struct ModResolver {
    builtin_modules: HashSet<String>,
}

impl ModResolver {
    fn new(builtin_modules: HashSet<String>) -> Self {
        Self { builtin_modules }
    }

    fn is_builtin(&self, name: &str) -> bool {
        self.builtin_modules.contains(name)
    }
}

impl Resolver for ModResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        if self.is_builtin(name) {
            return Ok(name.to_string());
        }
        if !name.starts_with('.') {
            return Err(Error::new_resolving(base, name));
        }

        normalize_relative_module(base, name)
            .ok_or_else(|| Error::new_resolving_message(base, name, "module escapes mod root"))
    }
}

#[derive(Debug)]
struct ModLoader {
    context: BridgeModContext,
    builtin_modules: HashMap<String, String>,
}

impl ModLoader {
    fn new(context: BridgeModContext, builtin_modules: HashMap<String, String>) -> Self {
        Self {
            context,
            builtin_modules,
        }
    }
}

impl Loader for ModLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        if let Some(source) = self.builtin_modules.get(name) {
            return Module::declare(ctx.clone(), name, source.as_bytes().to_vec());
        }

        let Some(path) = resolve_existing_script(&self.context, name) else {
            return Err(Error::new_loading(name));
        };
        let source = vm::source::read_script(&self.context, &path)
            .map_err(|error| Error::new_loading_message(name, error.to_string()))?;
        let source = vm::source::transpile_script(&path, &source)
            .map_err(|error| Error::new_loading_message(name, error))?;
        Module::declare(ctx.clone(), name, source)
    }
}

fn normalize_relative_module(base: &str, name: &str) -> Option<String> {
    if name.starts_with('/') || base.starts_with('/') {
        return None;
    }

    let mut parts = base
        .rsplit_once('/')
        .map(|(dir, _)| dir.split('/').collect::<Vec<_>>())
        .unwrap_or_default();

    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            segment => parts.push(segment),
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn resolve_existing_script(context: &BridgeModContext, name: &str) -> Option<String> {
    candidate_script_paths(name)
        .into_iter()
        .find(|candidate| vm::source::script_exists(context, candidate))
}

fn candidate_script_paths(name: &str) -> Vec<String> {
    let mut candidates = vec![name.to_string()];
    if Path::new(name).extension().is_none() {
        for extension in ["js", "ts", "mjs", "mts"] {
            candidates.push(format!("{name}.{extension}"));
            candidates.push(format!("{name}/index.{extension}"));
        }
    }
    candidates
}
