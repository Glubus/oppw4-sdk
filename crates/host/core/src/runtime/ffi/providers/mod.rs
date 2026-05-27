use std::ffi::CStr;

use plugin_abi::{
    Oppw4FileProvider, Oppw4ProviderCloseFn, Oppw4ProviderOpenPathFn, Oppw4ProviderReadFn,
    Oppw4ProviderSeekFn, Oppw4ProviderSizeFn,
};

mod r#unsafe;

pub(crate) use r#unsafe::host_register_file_provider;

struct RequiredProviderFns {
    open_path: Oppw4ProviderOpenPathFn,
    read: Oppw4ProviderReadFn,
    close: Oppw4ProviderCloseFn,
    size: Oppw4ProviderSizeFn,
    seek: Oppw4ProviderSeekFn,
}

fn register_file_provider(provider: &Oppw4FileProvider, plugin_id: Option<&CStr>) -> i32 {
    let Some(required) = required_provider_fns(provider) else {
        return -2;
    };
    if let Some(result) = crate::runtime::loader_services::register_file_provider(provider) {
        return result;
    }
    hooks::register_file_provider(hooks::FileProviderRegistration {
        plugin_id,
        provider_context: provider.provider_context,
        open_path: required.open_path,
        read: required.read,
        close: required.close,
        size: required.size,
        file_time: provider.file_time,
        seek: required.seek,
        patch_read: provider.patch_read,
    })
}

fn required_provider_fns(provider: &Oppw4FileProvider) -> Option<RequiredProviderFns> {
    Some(RequiredProviderFns {
        open_path: provider.open_path?,
        read: provider.read?,
        close: provider.close?,
        size: provider.size?,
        seek: provider.seek?,
    })
}
