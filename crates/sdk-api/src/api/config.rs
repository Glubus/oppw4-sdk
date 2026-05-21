use plugin_abi::{Oppw4ConfigSchema, Oppw4PluginApi};

use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct ConfigService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> ConfigService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn register_schema(
        self,
        plugin_id: &str,
        schema_name: &str,
        schema: &str,
    ) -> PluginResult<()> {
        let register = self
            .abi
            .register_config_schema
            .ok_or(PluginError::MissingHostFunction("register_config_schema"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let schema_name = cstring_lossy(schema_name);
        let schema = cstring_lossy(schema);
        let descriptor = Oppw4ConfigSchema {
            plugin_id: plugin_id.as_ptr(),
            schema_name: schema_name.as_ptr(),
            schema_utf8: schema.as_ptr(),
        };
        let code = r#unsafe::register_config_schema(self.abi.host_context, register, &descriptor);
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_config_schema",
                code,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{c_void, CStr};

    use plugin_abi::null_api;

    use super::*;

    unsafe extern "system" fn register_config_schema(
        _host_context: *mut c_void,
        schema: *const Oppw4ConfigSchema,
    ) -> i32 {
        let Some(schema) = schema.as_ref() else {
            return -1;
        };
        let plugin_id = CStr::from_ptr(schema.plugin_id).to_string_lossy();
        let name = CStr::from_ptr(schema.schema_name).to_string_lossy();
        let schema_text = CStr::from_ptr(schema.schema_utf8).to_string_lossy();
        if plugin_id == "fx_director" && name == "config" && schema_text.contains("[config]") {
            0
        } else {
            -1
        }
    }

    #[test]
    fn registers_config_schema() {
        let mut api = null_api();
        api.register_config_schema = Some(register_config_schema);

        let result =
            ConfigService::new(&api).register_schema("fx_director", "config", "[config]\n");

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn reports_schema_registration_failure() {
        unsafe extern "system" fn reject_schema(
            _host_context: *mut c_void,
            _schema: *const Oppw4ConfigSchema,
        ) -> i32 {
            -28
        }

        let mut api = null_api();
        api.register_config_schema = Some(reject_schema);

        let error = ConfigService::new(&api)
            .register_schema("fx_director", "config", "[config]\n")
            .expect_err("registration should fail");

        assert_eq!(
            error,
            PluginError::HostCallFailed {
                operation: "register_config_schema",
                code: -28
            }
        );
    }
}
