use plugin_abi::Oppw4PluginApi;

#[derive(Clone, Copy)]
pub struct RankService<'api> {
    _abi: &'api Oppw4PluginApi,
}

impl<'api> RankService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { _abi: abi }
    }
}
