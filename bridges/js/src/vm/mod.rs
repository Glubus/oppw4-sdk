mod api;
mod handlers;
mod loader;
mod modules;
mod runtime;
mod source;

pub use loader::load;

use sdk_bridge::{EventEnvelope, HandlerDescriptor};

pub struct JsVm {
    runtime: rquickjs::Runtime,
    context: rquickjs::Context,
    handler_descriptors: Vec<HandlerDescriptor>,
}

impl JsVm {
    pub(super) fn new(
        runtime: rquickjs::Runtime,
        context: rquickjs::Context,
        handler_descriptors: Vec<HandlerDescriptor>,
    ) -> Self {
        Self {
            runtime,
            context,
            handler_descriptors,
        }
    }

    pub fn handler_descriptors(&self) -> &[HandlerDescriptor] {
        &self.handler_descriptors
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
