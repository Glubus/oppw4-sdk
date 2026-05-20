use crate::{r#unsafe, Oppw4PluginApi, OPPW4_PLUGIN_API_STRUCT_SIZE, OPPW4_PLUGIN_API_VERSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginInitError {
    NullApi,
    InvalidApiVersion { expected: u32, actual: u32 },
    ApiStructTooSmall { expected: u32, actual: u32 },
}

impl PluginInitError {
    pub const fn code(self) -> i32 {
        match self {
            Self::NullApi => -1,
            Self::InvalidApiVersion { .. } => -2,
            Self::ApiStructTooSmall { .. } => -4,
        }
    }
}

pub fn validate_plugin_api(api: &Oppw4PluginApi) -> Result<(), PluginInitError> {
    if api.version != OPPW4_PLUGIN_API_VERSION {
        return Err(PluginInitError::InvalidApiVersion {
            expected: OPPW4_PLUGIN_API_VERSION,
            actual: api.version,
        });
    }
    if api.struct_size < OPPW4_PLUGIN_API_STRUCT_SIZE {
        return Err(PluginInitError::ApiStructTooSmall {
            expected: OPPW4_PLUGIN_API_STRUCT_SIZE,
            actual: api.struct_size,
        });
    }
    Ok(())
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
    validate_plugin_api(api)?;
    Ok(api)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_abi::null_api;

    #[test]
    fn accepts_current_or_larger_struct_size() {
        let mut api = null_api();
        validate_plugin_api(&api).expect("current");

        api.struct_size = api.struct_size.saturating_add(64);
        validate_plugin_api(&api).expect("larger");
    }

    #[test]
    fn rejects_too_small_struct_size() {
        let mut api = null_api();
        api.struct_size = api.struct_size.saturating_sub(1);

        let error = validate_plugin_api(&api).expect_err("too small");

        assert_eq!(
            error,
            PluginInitError::ApiStructTooSmall {
                expected: OPPW4_PLUGIN_API_STRUCT_SIZE,
                actual: api.struct_size,
            }
        );
    }
}
