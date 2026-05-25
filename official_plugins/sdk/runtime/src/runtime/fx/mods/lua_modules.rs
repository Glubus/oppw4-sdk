use std::{ffi::c_void, sync::Arc};

use mlua::Lua;
use plugin_sdk::{HostApi, PluginError};

use crate::runtime::fx::log;

use super::{character_ext::runtime_fx_module, state::SharedFxState};

const MODULE_OWNER: &str = "sdk_runtime";
const MODULE_NAME: &str = "sdk.runtime.fx";

pub(crate) fn register_lua_modules(host: HostApi<'_>, state: SharedFxState) {
    register_module(
        host,
        MODULE_OWNER,
        MODULE_NAME,
        register_runtime_fx_module,
        state,
    );
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
            "sdk.runtime.fx lua module register failed module={name} result={result}"
        ));
    }
}

unsafe extern "system" fn register_runtime_fx_module(
    context: *mut c_void,
    lua: *mut c_void,
) -> i32 {
    register_named_module(context, lua)
}

unsafe fn register_named_module(context: *mut c_void, lua: *mut c_void) -> i32 {
    let Some(state) = context.cast::<SharedFxState>().as_ref() else {
        return -1;
    };
    let Some(lua) = lua.cast::<Lua>().as_ref() else {
        return -2;
    };
    match runtime_fx_module(lua, Arc::clone(state))
        .and_then(|table| lua_api::register_module(lua, MODULE_NAME, table))
    {
        Ok(()) => 0,
        Err(error) => {
            log::write_line(format!(
                "sdk.runtime.fx lua module failed name={MODULE_NAME}: {error}"
            ));
            -3
        }
    }
}
