use std::ffi::c_void;

use crate::{
    HostApi, HostRdbPatchReadFn, Oppw4LuaRegisterFn, PluginContext, PluginResult,
    VirtualFileProvider,
};

pub trait PluginFeature {
    fn id(&self) -> &'static str;

    fn required_capabilities(&self) -> &'static [&'static str] {
        &[]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()>;
}

pub struct PluginRegistrar<'api> {
    context: PluginContext<'api>,
}

impl<'api> PluginRegistrar<'api> {
    pub const fn new(context: PluginContext<'api>) -> Self {
        Self { context }
    }

    pub const fn context(&self) -> PluginContext<'api> {
        self.context
    }

    pub const fn host(&self) -> HostApi<'api> {
        self.context.host()
    }

    pub const fn plugin_id(&self) -> &'static str {
        self.context.plugin_id()
    }

    pub fn add<F>(&mut self, feature: F) -> PluginResult<&mut Self>
    where
        F: PluginFeature,
    {
        self.require_all(feature.required_capabilities())?;
        feature.install(self)?;
        self.context
            .log(format!("feature installed: {}", feature.id()));
        Ok(self)
    }

    pub fn finish(self) -> PluginResult<()> {
        Ok(())
    }

    pub fn require_all(&self, capabilities: &[&str]) -> PluginResult<()> {
        for capability in capabilities {
            self.host()
                .capabilities()
                .require(self.plugin_id(), capability)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LuaModuleFeature {
    id: &'static str,
    module_name: &'static str,
    module_context: *mut c_void,
    register: Oppw4LuaRegisterFn,
}

impl LuaModuleFeature {
    pub const fn new(
        id: &'static str,
        module_name: &'static str,
        register: Oppw4LuaRegisterFn,
    ) -> Self {
        Self {
            id,
            module_name,
            module_context: std::ptr::null_mut(),
            register,
        }
    }

    pub const fn context(mut self, module_context: *mut c_void) -> Self {
        self.module_context = module_context;
        self
    }

    pub const fn module_name(&self) -> &'static str {
        self.module_name
    }
}

impl PluginFeature for LuaModuleFeature {
    fn id(&self) -> &'static str {
        self.id
    }

    fn required_capabilities(&self) -> &'static [&'static str] {
        &["lua.module"]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()> {
        registrar.host().lua().register_module_fn(
            registrar.plugin_id(),
            self.module_name,
            self.module_context,
            self.register,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConfigFeature {
    id: &'static str,
    schema_name: &'static str,
    schema_toml: &'static str,
}

impl ConfigFeature {
    pub const fn new(
        id: &'static str,
        schema_name: &'static str,
        schema_toml: &'static str,
    ) -> Self {
        Self {
            id,
            schema_name,
            schema_toml,
        }
    }

    pub const fn schema_name(&self) -> &'static str {
        self.schema_name
    }
}

impl PluginFeature for ConfigFeature {
    fn id(&self) -> &'static str {
        self.id
    }

    fn required_capabilities(&self) -> &'static [&'static str] {
        &["config.schema"]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()> {
        registrar.host().configs().register_schema(
            registrar.plugin_id(),
            self.schema_name,
            self.schema_toml,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualFileProviderFeature<'a> {
    id: &'static str,
    provider: VirtualFileProvider<'a>,
}

impl<'a> VirtualFileProviderFeature<'a> {
    pub const fn new(id: &'static str, provider: VirtualFileProvider<'a>) -> Self {
        Self { id, provider }
    }
}

impl PluginFeature for VirtualFileProviderFeature<'_> {
    fn id(&self) -> &'static str {
        self.id
    }

    fn required_capabilities(&self) -> &'static [&'static str] {
        &["files.virtualize"]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()> {
        registrar
            .host()
            .files()
            .register_virtual_provider(self.provider)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RdbPatchCallbackFeature {
    id: &'static str,
    provider_context: *mut c_void,
    patch_read: HostRdbPatchReadFn,
}

impl RdbPatchCallbackFeature {
    pub const fn new(id: &'static str, patch_read: HostRdbPatchReadFn) -> Self {
        Self {
            id,
            provider_context: std::ptr::null_mut(),
            patch_read,
        }
    }

    pub const fn context(mut self, provider_context: *mut c_void) -> Self {
        self.provider_context = provider_context;
        self
    }
}

impl PluginFeature for RdbPatchCallbackFeature {
    fn id(&self) -> &'static str {
        self.id
    }

    fn required_capabilities(&self) -> &'static [&'static str] {
        &["rdb.patch"]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()> {
        unsafe {
            registrar
                .host()
                .rdb()
                .register_patch_provider(self.provider_context, self.patch_read)
        }
    }
}

pub trait RdbPatchFeature {
    fn id(&self) -> &'static str;
}

pub trait RdbVirtualFeature {
    fn id(&self) -> &'static str;
}

pub trait LinkDataPatchFeature {
    fn id(&self) -> &'static str;
}

pub trait SignalFeature {
    fn id(&self) -> &'static str;
    fn subscriptions(&self) -> &'static [&'static str];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Plugin, PluginResult};
    use plugin_abi::null_api;
    use std::ffi::{c_char, c_void};

    struct TestPlugin;

    impl Plugin for TestPlugin {
        const ID: &'static str = "test_plugin";

        fn init(_context: PluginContext<'_>) -> PluginResult<()> {
            Ok(())
        }
    }

    struct TestFeature;

    impl PluginFeature for TestFeature {
        fn id(&self) -> &'static str {
            "test_feature"
        }

        fn install(&self, _registrar: &mut PluginRegistrar<'_>) -> PluginResult<()> {
            Ok(())
        }
    }

    #[test]
    fn registrar_installs_feature() {
        let api = null_api();
        let context = PluginContext::new::<TestPlugin>(&api).expect("context");
        let mut registrar = PluginRegistrar::new(context);

        let result = registrar.add(TestFeature);

        assert!(result.is_ok());
    }

    unsafe extern "system" fn register_lua(
        _module_context: *mut c_void,
        _lua_context: *mut c_void,
    ) -> i32 {
        0
    }

    unsafe extern "system" fn patch_read(
        _provider_context: *mut c_void,
        _path_utf8: *const c_char,
        _os_handle: usize,
        _read_offset: u64,
        _buffer: *mut u8,
        _len: usize,
    ) -> i32 {
        0
    }

    crate::sdk_plugin! {
        id = "macro_plugin",
        name = "Macro Plugin",
        features = [],
    }

    #[test]
    fn macro_helpers_construct_features() {
        let lua = crate::lua_module! {
            plugin = "macro_plugin",
            module = "macro_plugin",
            register = register_lua,
        };
        assert_eq!(lua.module_name(), "macro_plugin");

        let config = crate::config_schema! {
            name = "config",
            toml = "[config]\n",
        };
        assert_eq!(config.schema_name(), "config");

        let rdb = crate::rdb_patch_feature! {
            id = "macro_plugin.rdb",
            patch_read = patch_read,
        };
        assert_eq!(PluginFeature::id(&rdb), "macro_plugin.rdb");
    }
}
