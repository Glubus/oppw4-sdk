use std::{ffi::c_void, sync::Arc};

use mlua::Lua;
use plugin_sdk::{HostApi, PluginError};

use crate::log;

use super::{character_ext::fx_director_module, fx_module::fx_module, state::SharedFxState};

pub(crate) fn register_lua_modules(host: HostApi<'_>, state: SharedFxState) {
    register_module(
        host,
        "fx_director",
        "fx_director",
        register_fx_director_module,
        Arc::clone(&state),
    );
    register_module(host, "fx_director", "fx", register_fx_module, state);
}

fn register_module(
    host: HostApi<'_>,
    plugin_id: &str,
    name: &str,
    register: plugin_sdk::Oppw4LuaRegisterFn,
    state: SharedFxState,
) {
    let context = Box::into_raw(Box::new(state)).cast::<c_void>();
    let result = match host
        .lua()
        .register_module_fn(plugin_id, name, context, register)
    {
        Ok(()) => 0,
        Err(PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };
    if result != 0 {
        unsafe {
            drop(Box::from_raw(context.cast::<SharedFxState>()));
        }
        log::write_line(format!(
            "fx_director lua module register failed module={name} result={result}"
        ));
    }
}

unsafe extern "system" fn register_fx_director_module(
    context: *mut c_void,
    lua: *mut c_void,
) -> i32 {
    register_named_module(context, lua, ModuleKind::Director)
}

unsafe extern "system" fn register_fx_module(context: *mut c_void, lua: *mut c_void) -> i32 {
    register_named_module(context, lua, ModuleKind::Fx)
}

enum ModuleKind {
    Director,
    Fx,
}

unsafe fn register_named_module(context: *mut c_void, lua: *mut c_void, kind: ModuleKind) -> i32 {
    let Some(state) = context.cast::<SharedFxState>().as_ref() else {
        return -1;
    };
    let Some(lua) = lua.cast::<Lua>().as_ref() else {
        return -2;
    };
    let (name, module) = match kind {
        ModuleKind::Director => ("fx_director", fx_director_module(lua, Arc::clone(state))),
        ModuleKind::Fx => ("fx", fx_module(lua, Arc::clone(state))),
    };
    match module.and_then(|table| lua_api::register_module(lua, name, table)) {
        Ok(()) => 0,
        Err(error) => {
            log::write_line(format!(
                "fx_director lua module failed name={name}: {error}"
            ));
            -3
        }
    }
}
