use std::ffi::c_void;

use plugin_abi::{
    Oppw4FileProvider, Oppw4PluginApi, Oppw4ProviderCloseFn, Oppw4ProviderFileTimeFn,
    Oppw4ProviderOpenPathFn, Oppw4ProviderPatchReadFn, Oppw4ProviderReadFn, Oppw4ProviderSeekFn,
    Oppw4ProviderSizeFn,
};

use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct FileService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> FileService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn register_provider(self, provider: &Oppw4FileProvider) -> PluginResult<()> {
        let register = self
            .abi
            .register_file_provider
            .ok_or(PluginError::MissingHostFunction("register_file_provider"))?;
        let code = r#unsafe::register_file_provider(self.abi.host_context, register, provider);
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_file_provider",
                code,
            })
        }
    }

    pub fn register_virtual_provider(self, provider: VirtualFileProvider<'_>) -> PluginResult<()> {
        let plugin_id = cstring_lossy(provider.plugin_id);
        let raw = provider.into_raw(plugin_id.as_ptr());
        self.register_provider(&raw)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualFileProvider<'a> {
    plugin_id: &'a str,
    provider_context: *mut c_void,
    open_path: Oppw4ProviderOpenPathFn,
    read: Oppw4ProviderReadFn,
    close: Oppw4ProviderCloseFn,
    size: Oppw4ProviderSizeFn,
    file_time: Option<Oppw4ProviderFileTimeFn>,
    seek: Option<Oppw4ProviderSeekFn>,
    patch_read: Option<Oppw4ProviderPatchReadFn>,
}

impl<'a> VirtualFileProvider<'a> {
    pub const fn new(
        plugin_id: &'a str,
        open_path: Oppw4ProviderOpenPathFn,
        read: Oppw4ProviderReadFn,
        close: Oppw4ProviderCloseFn,
        size: Oppw4ProviderSizeFn,
    ) -> Self {
        Self {
            plugin_id,
            provider_context: std::ptr::null_mut(),
            open_path,
            read,
            close,
            size,
            file_time: None,
            seek: None,
            patch_read: None,
        }
    }

    pub const fn context(mut self, provider_context: *mut c_void) -> Self {
        self.provider_context = provider_context;
        self
    }

    pub(crate) const fn plugin_id(&self) -> &'a str {
        self.plugin_id
    }

    pub const fn file_time(mut self, file_time: Oppw4ProviderFileTimeFn) -> Self {
        self.file_time = Some(file_time);
        self
    }

    pub const fn seek(mut self, seek: Oppw4ProviderSeekFn) -> Self {
        self.seek = Some(seek);
        self
    }

    pub const fn patch_read(mut self, patch_read: Oppw4ProviderPatchReadFn) -> Self {
        self.patch_read = Some(patch_read);
        self
    }

    pub(crate) fn into_raw(self, plugin_id: *const std::ffi::c_char) -> Oppw4FileProvider {
        Oppw4FileProvider {
            plugin_id,
            provider_context: self.provider_context,
            open_path: Some(self.open_path),
            read: Some(self.read),
            close: Some(self.close),
            size: Some(self.size),
            file_time: self.file_time,
            seek: self.seek,
            patch_read: self.patch_read,
        }
    }
}

#[cfg(test)]
mod tests;
