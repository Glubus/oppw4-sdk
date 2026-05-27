use plugin_abi::Oppw4PluginApi;

#[derive(Clone, Copy)]
pub struct DifficultyService<'api> {
    _abi: &'api Oppw4PluginApi,
}

impl<'api> DifficultyService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { _abi: abi }
    }
}
