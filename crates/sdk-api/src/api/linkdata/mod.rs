mod entry;
mod row;
mod target;

use plugin_abi::Oppw4PluginApi;

pub use target::LinkDataRowTarget;

#[derive(Clone, Copy)]
pub struct LinkDataService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> LinkDataService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }
}
