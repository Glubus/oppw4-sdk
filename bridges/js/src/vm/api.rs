use rquickjs::Ctx;

const BOOTSTRAP_JS: &str = concat!(
    include_str!("bootstrap/start.js"),
    "\n",
    include_str!("bootstrap/globals.js"),
    "\n",
    include_str!("bootstrap/events.js"),
    "\n",
    include_str!("bootstrap/registry.js"),
    "\n",
    include_str!("bootstrap/contexts.js"),
    "\n",
    include_str!("bootstrap/invoke.js"),
    "\n",
    include_str!("bootstrap/wrap.js"),
    "\n",
    include_str!(concat!(env!("OUT_DIR"), "/generated_runtime_projection.js")),
    "\n",
    include_str!("bootstrap/extensions.js"),
    "\n",
    include_str!("bootstrap/end.js"),
);

pub(super) fn install(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    ctx.eval::<(), _>(BOOTSTRAP_JS)
}
