mod config;
mod hooks;
mod log;
mod memory;
mod mods;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginError, PluginResult};

struct FxDirector;

impl Plugin for FxDirector {
    const ID: &'static str = "fx_director";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        log::initialize(host);
        let shared_config = mods::load_config(host);
        mods::register_lua_modules(host, shared_config.clone());
        let plugin = shared_config.lock().expect("fx state lock").plugin_config();
        log::write_line(format!(
            "fx_director init trigger={:?} hotkey_vk=0x{:02x} observe_effect_ids={} observe_character_probe={} install_delay_ms={} wait_for={:?} refresh_interval_ms={}",
            plugin.trigger,
            plugin.hotkey_vk,
            plugin.debug.observe_effect_ids,
            plugin.debug.observe_character_probe,
            plugin.install_delay_ms,
            plugin.wait_for,
            plugin.refresh_interval_ms
        ));
        let code = hooks::install_deferred(host.owned(), shared_config);
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "fx_director_install_deferred",
                code,
            })
        }
    }
}

export_plugin!(FxDirector);

pub extern "system" fn oppw4_fx_director_set_enabled(enabled: i32) -> i32 {
    hooks::set_enabled(enabled != 0)
}

#[no_mangle]
pub extern "system" fn oppw4_fx_director_set_effect_id(effect_id: u32) -> i32 {
    hooks::set_effect_id(effect_id)
}

#[no_mangle]
pub extern "system" fn oppw4_fx_director_set_timing(
    animation_speed: f32,
    loop_start: f32,
    loop_end: f32,
) -> i32 {
    hooks::set_timing(animation_speed, loop_start, loop_end)
}
