use std::sync::{Arc, Mutex};

use rquickjs::{prelude::Func, Ctx};
use sdk_bridge::ModId;

pub(super) fn install(
    ctx: Ctx<'_>,
    mod_id: &ModId,
    logs: Arc<Mutex<Vec<String>>>,
) -> rquickjs::Result<()> {
    let mod_id = mod_id.as_str().to_string();
    ctx.globals().set(
        "__oppw4_trace",
        Func::from(move |message: String| {
            if let Ok(mut logs) = logs.lock() {
                logs.push(format!("js trace mod={mod_id} {message}"));
            }
        }),
    )
}
