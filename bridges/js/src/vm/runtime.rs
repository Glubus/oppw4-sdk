use rquickjs::Ctx;
use sdk_bridge::{BridgeModContext, BridgeModSource};

pub(super) fn install_mod_globals(
    ctx: Ctx<'_>,
    context: &BridgeModContext,
) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    globals.set("__oppw4_mod_id", context.mod_id.as_str())?;
    globals.set("__oppw4_mod_name", context.name.as_str())?;
    globals.set(
        "__oppw4_mod_root",
        match &context.source {
            BridgeModSource::Directory(root) => root.to_string_lossy().to_string(),
            BridgeModSource::Zip { path, .. } => path.to_string_lossy().to_string(),
        },
    )?;
    globals.set(
        "__oppw4_mod_zip_root",
        match &context.source {
            BridgeModSource::Directory(_) => String::new(),
            BridgeModSource::Zip { root, .. } => root.clone(),
        },
    )?;
    globals.set(
        "__oppw4_mod_is_zip",
        matches!(context.source, BridgeModSource::Zip { .. }),
    )
}

pub(super) fn hide_unsafe_globals(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    globals.set("std", rquickjs::Null)?;
    globals.set("os", rquickjs::Null)
}
