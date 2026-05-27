use std::{
    collections::HashMap,
    ffi::{c_char, c_void},
    sync::{Mutex, OnceLock},
};

use plugin_abi::{optional_cstr, Oppw4ConfigSchema};

use crate::runtime::ffi::{context_from_raw, CAP_CONFIG_SCHEMA};

static CONFIG_SCHEMAS: OnceLock<Mutex<HashMap<ConfigSchemaKey, String>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ConfigSchemaKey {
    plugin_id: String,
    schema_name: String,
}

pub(crate) unsafe extern "system" fn host_register_config_schema(
    host_context: *mut c_void,
    schema: *const Oppw4ConfigSchema,
) -> i32 {
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let Some(schema) = schema.as_ref() else {
        return -1;
    };
    if let Err(code) =
        context.require_capability_for_cstr(optional_cstr(schema.plugin_id), CAP_CONFIG_SCHEMA)
    {
        return code;
    }
    let Some(schema_name) = non_empty_cstr(schema.schema_name) else {
        return -26;
    };
    let Some(schema_body) = non_empty_cstr(schema.schema_utf8) else {
        return -27;
    };
    let key = ConfigSchemaKey {
        plugin_id: context.plugin_id().to_ascii_lowercase(),
        schema_name: schema_name.to_ascii_lowercase(),
    };
    let Ok(mut schemas) = registry().lock() else {
        return -3;
    };
    if schemas.contains_key(&key) {
        return -28;
    }
    schemas.insert(key, schema_body);
    0
}

fn registry() -> &'static Mutex<HashMap<ConfigSchemaKey, String>> {
    CONFIG_SCHEMAS.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn non_empty_cstr(raw: *const c_char) -> Option<String> {
    let value = unsafe { optional_cstr(raw) }?
        .to_string_lossy()
        .trim()
        .to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ffi::ApiContext;

    #[test]
    fn config_schema_requires_capability() {
        let context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );
        let schema = schema("fx_director", "config", "[config]\n");

        let code = unsafe {
            host_register_config_schema((&context as *const ApiContext).cast_mut().cast(), &schema)
        };

        assert_eq!(code, -22);
    }

    #[test]
    fn config_schema_rejects_plugin_id_spoofing() {
        let context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            [CAP_CONFIG_SCHEMA.to_string()],
            Vec::<String>::new(),
        );
        let schema = schema("other_plugin", "config", "[config]\n");

        let code = unsafe {
            host_register_config_schema((&context as *const ApiContext).cast_mut().cast(), &schema)
        };

        assert_eq!(code, -21);
    }

    #[test]
    fn config_schema_rejects_missing_name() {
        let context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            [CAP_CONFIG_SCHEMA.to_string()],
            Vec::<String>::new(),
        );
        let schema = schema("fx_director", "", "[config]\n");

        let code = unsafe {
            host_register_config_schema((&context as *const ApiContext).cast_mut().cast(), &schema)
        };

        assert_eq!(code, -26);
    }

    fn schema(
        plugin_id: &'static str,
        schema_name: &'static str,
        schema: &'static str,
    ) -> Oppw4ConfigSchema {
        Oppw4ConfigSchema {
            plugin_id: cstr(plugin_id).as_ptr(),
            schema_name: cstr(schema_name).as_ptr(),
            schema_utf8: cstr(schema).as_ptr(),
        }
    }

    fn cstr(value: &'static str) -> &'static std::ffi::CStr {
        match value {
            "fx_director" => c"fx_director",
            "other_plugin" => c"other_plugin",
            "config" => c"config",
            "" => c"",
            "[config]\n" => c"[config]\n",
            _ => unreachable!("unexpected test cstr"),
        }
    }
}
