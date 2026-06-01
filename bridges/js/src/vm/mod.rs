mod api;
mod error;
mod handlers;
mod loader;
mod modules;
mod runtime;
mod source;

pub use loader::load;

use sdk_bridge::{
    BridgeAnalysisReport, EventEnvelope, HandlerDescriptor, ModId, MutationEnvelope, MutationKey,
};
use serde::Deserialize;
use std::sync::mpsc::Receiver;

use self::error::StringContext;

pub struct JsVm {
    runtime: rquickjs::Runtime,
    context: rquickjs::Context,
    handler_descriptors: Vec<HandlerDescriptor>,
    logs: Receiver<String>,
    analysis: BridgeAnalysisReport,
}

impl JsVm {
    pub(super) fn new(
        runtime: rquickjs::Runtime,
        context: rquickjs::Context,
        handler_descriptors: Vec<HandlerDescriptor>,
        logs: Receiver<String>,
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
        self.logs.try_iter().collect()
    }

    pub fn dispatch(
        &self,
        handler: &HandlerDescriptor,
        event: &EventEnvelope,
    ) -> Result<Vec<MutationEnvelope>, String> {
        self.dispatch_many(&[handler], event)
    }

    pub fn dispatch_many(
        &self,
        handlers: &[&HandlerDescriptor],
        event: &EventEnvelope,
    ) -> Result<Vec<MutationEnvelope>, String> {
        let _keep_runtime_alive = &self.runtime;
        let source_mod = handlers
            .first()
            .map(|handler| handler.mod_id.clone())
            .ok_or_else(|| "js dispatch requires at least one handler".to_string())?;
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
                .call::<_, String>((
                    handler_refs,
                    event.key.as_str().to_string(),
                    event.payload_json.as_ref(),
                ))
                .context("js handler call failed")
                .and_then(|json| parse_mutations(&source_mod, &json))
        })
    }
}

#[derive(Deserialize)]
struct JsMutation {
    key: String,
    payload: serde_json::Value,
}

fn parse_mutations(source_mod: &ModId, json: &str) -> Result<Vec<MutationEnvelope>, String> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mutations =
        serde_json::from_str::<Vec<JsMutation>>(json).map_err(|error| error.to_string())?;
    mutations
        .into_iter()
        .map(|mutation| {
            let key = MutationKey::new(mutation.key).map_err(|error| format!("{error:?}"))?;
            Ok(MutationEnvelope::new(
                key,
                source_mod.clone(),
                mutation.payload.to_string(),
            ))
        })
        .collect()
}
