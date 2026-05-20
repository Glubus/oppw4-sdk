use crate::{r#unsafe, Oppw4PluginApi, OPPW4_PLUGIN_API_VERSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginInitError {
    NullApi,
    InvalidApiVersion { expected: u32, actual: u32 },
}

impl PluginInitError {
    pub const fn code(self) -> i32 {
        match self {
            Self::NullApi => -1,
            Self::InvalidApiVersion { .. } => -2,
        }
    }
}

/// # Safety
///
/// `api` must either be null or point to a valid `Oppw4PluginApi` table for the
/// duration of plugin initialization. The returned reference must not outlive
/// the host-owned table.
pub unsafe fn plugin_abi_from_raw(
    api: *const Oppw4PluginApi,
) -> Result<&'static Oppw4PluginApi, PluginInitError> {
    let api = r#unsafe::abi_ref(api).ok_or(PluginInitError::NullApi)?;
    if api.version != OPPW4_PLUGIN_API_VERSION {
        return Err(PluginInitError::InvalidApiVersion {
            expected: OPPW4_PLUGIN_API_VERSION,
            actual: api.version,
        });
    }
    Ok(api)
}
