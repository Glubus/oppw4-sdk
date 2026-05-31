use rquickjs::Ctx;

const BOOTSTRAP_JS: &str = include_str!("bootstrap.js");

pub(super) fn install(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    ctx.eval::<(), _>(BOOTSTRAP_JS)
}
