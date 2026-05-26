use std::path::{Path, PathBuf};

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

mod character;

struct SdkData;

impl Plugin for SdkData {
    const ID: &'static str = "sdk_data";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        initialize_data(context);
        register_character_module(context);
        Ok(())
    }
}

fn initialize_data(context: PluginContext<'_>) {
    let Some(game_root) = context.game_root() else {
        struct_api::mark_data_unavailable();
        context.log("sdk.data unavailable: game root is missing");
        return;
    };
    let data_root = data_root(&game_root);
    match struct_api::initialize_data_root(&data_root) {
        Ok(()) => context.log(format!(
            "sdk.data loaded OPPW4 data from {}",
            data_root.display()
        )),
        Err(error) => {
            struct_api::mark_data_unavailable();
            context.log(format!(
                "sdk.data unavailable at {}: {error:?}",
                data_root.display()
            ));
        }
    }
}

fn data_root(game_root: &Path) -> PathBuf {
    game_root.join("oppw4-data")
}

fn register_character_module(context: PluginContext<'_>) {
    match context.host().lua().register_module_fn(
        SdkData::ID,
        "std.character",
        std::ptr::null_mut(),
        register_character_lua_module,
    ) {
        Ok(()) => context.log("sdk.data registered std.character"),
        Err(error) => context.log(format!("sdk.data std.character register failed: {error}")),
    }
}

unsafe extern "system" fn register_character_lua_module(
    _context: *mut std::ffi::c_void,
    lua: *mut std::ffi::c_void,
) -> i32 {
    let Some(lua) = (unsafe { lua.cast::<mlua::Lua>().as_ref() }) else {
        return -1;
    };
    match character::install(lua) {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

export_plugin!(SdkData);
