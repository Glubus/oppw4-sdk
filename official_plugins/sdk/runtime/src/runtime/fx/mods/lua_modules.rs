use crate::runtime::lua_module;

use super::{character_ext::runtime_fx_module, state::SharedFxState};

const MODULE_NAME: &str = "sdk.runtime.fx";

lua_module::runtime_lua_module! {
    type = RuntimeFxLuaModule,
    module = MODULE_NAME,
    context = SharedFxState,
    factory = runtime_fx_module,
}
