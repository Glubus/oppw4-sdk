mod api;
mod error;
mod handlers;
mod loader;
mod modules;
mod runtime;
mod source;

pub use loader::load;

use sdk_bridge::{BridgeAnalysisReport, EventEnvelope, HandlerDescriptor};
use std::sync::{Arc, Mutex};

use self::error::StringContext;

pub struct JsVm {
    runtime: rquickjs::Runtime,
    context: rquickjs::Context,
    handler_descriptors: Vec<HandlerDescriptor>,
    logs: Arc<Mutex<Vec<String>>>,
    analysis: BridgeAnalysisReport,
}

impl JsVm {
    pub(super) fn new(
        runtime: rquickjs::Runtime,
        context: rquickjs::Context,
        handler_descriptors: Vec<HandlerDescriptor>,
        logs: Arc<Mutex<Vec<String>>>,
        analysis: BridgeAnalysisReport,
    ) -> Self {
        Self {
            runtime,
            context,
            handler_descriptors,
            logs,
            analysis,
        }
    }

    pub fn handler_descriptors(&self) -> &[HandlerDescriptor] {
        &self.handler_descriptors
    }

    pub fn analysis(&self) -> &BridgeAnalysisReport {
        &self.analysis
    }

    pub fn drain_logs(&self) -> Vec<String> {
        self.logs
            .lock()
            .map(|mut logs| std::mem::take(&mut *logs))
            .unwrap_or_default()
    }

    pub fn dispatch(
        &self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> Result<(), String> {
        self.dispatch_many(std::slice::from_ref(handler), event)
    }

    pub fn dispatch_many(
        &self,
        handlers: &[HandlerDescriptor],
        event: &EventEnvelope,
    ) -> Result<(), String> {
        let _keep_runtime_alive = &self.runtime;
        let handler_refs = handlers
            .iter()
            .map(|handler| handler.handler_ref.as_str().to_string())
            .collect::<Vec<_>>();
        self.context.with(|ctx| {
            let dispatch = ctx
                .globals()
                .get::<_, rquickjs::Function>("__oppw4_dispatch_handlers")
                .context("js dispatch lookup failed")?;
            dispatch
                .call::<_, ()>((
                    handler_refs,
                    event.key.as_str().to_string(),
                    event.payload_json.as_ref(),
                ))
                .context("js handler call failed")
        })
    }
}
