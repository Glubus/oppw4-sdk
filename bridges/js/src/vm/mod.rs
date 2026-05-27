mod api;
mod handlers;
mod loader;
mod modules;
mod runtime;
mod source;

pub use loader::load;

use sdk_bridge::{EventEnvelope, HandlerDescriptor};
use std::sync::{Arc, Mutex};

pub struct JsVm {
    runtime: rquickjs::Runtime,
    context: rquickjs::Context,
    handler_descriptors: Vec<HandlerDescriptor>,
    logs: Arc<Mutex<Vec<String>>>,
}

impl JsVm {
    pub(super) fn new(
        runtime: rquickjs::Runtime,
        context: rquickjs::Context,
        handler_descriptors: Vec<HandlerDescriptor>,
        logs: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            runtime,
            context,
            handler_descriptors,
            logs,
        }
    }

    pub fn handler_descriptors(&self) -> &[HandlerDescriptor] {
        &self.handler_descriptors
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
        let _keep_runtime_alive = &self.runtime;
        self.context.with(|ctx| {
            let dispatch = ctx
                .globals()
                .get::<_, rquickjs::Function>("__oppw4_dispatch_handler")
                .map_err(|error| format!("js dispatch lookup failed: {error}"))?;
            dispatch
                .call::<_, ()>((
                    handler.handler_ref.as_str().to_string(),
                    event.key.as_str().to_string(),
                    event.payload_json.clone(),
                ))
                .map_err(|error| format!("js handler call failed: {error}"))
        })
    }
}
