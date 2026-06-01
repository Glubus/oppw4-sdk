use std::sync::mpsc::Sender;

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::ModId;

pub(super) fn install(ctx: Ctx<'_>, mod_id: &ModId, logs: Sender<String>) -> rquickjs::Result<()> {
    let mod_id = mod_id.as_str().to_string();
    ctx.globals().set(
        "__oppw4_trace",
        Func::from(move |message: String| {
            let _ = logs.send(format!("js trace mod={mod_id} {message}"));
        }),
    )
}
