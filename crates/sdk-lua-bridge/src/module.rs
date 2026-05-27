use plugin_abi::Oppw4LuaRegisterFn;
use sdk_bridge::BridgeModuleLoad;

#[derive(Clone)]
pub struct LuaModule {
    pub plugin_id: String,
    pub module_name: String,
    pub context: usize,
    pub register: Oppw4LuaRegisterFn,
    pub load: BridgeModuleLoad,
}
