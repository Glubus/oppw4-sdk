use plugin_abi::Oppw4PluginApi;

pub(crate) fn abi_ref(api: *const Oppw4PluginApi) -> Option<&'static Oppw4PluginApi> {
    unsafe { api.as_ref() }
}
