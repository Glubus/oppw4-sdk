use plugin_sdk::{export_plugin, HostApi, Plugin, PluginContext, PluginError, PluginResult};

mod constants;
mod log;
mod state;

struct MovesetPatcher;

impl Plugin for MovesetPatcher {
    const ID: &'static str = constants::PLUGIN_ID;

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        initialize(context.host()).map_err(PluginError::from)
    }
}

export_plugin!(MovesetPatcher);

fn initialize(host: HostApi<'_>) -> Result<(), String> {
    log::init(host);
    state::initialize(host)?;
    let edits = state::edit_count();
    log::write(
        host,
        format!("moveset_patcher initialized entry_patches={edits}"),
    );
    Ok(())
}
