use std::ffi::c_void;

use plugin_abi::{optional_cstr, Oppw4FileProvider};

use crate::runtime::ffi::context::{context_from_raw, CAP_FILES_VIRTUALIZE, CAP_RDB_PATCH};

pub(crate) unsafe extern "system" fn host_register_file_provider(
    host_context: *mut c_void,
    provider: *const Oppw4FileProvider,
) -> i32 {
    let Some(provider) = provider.as_ref() else {
        return -1;
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) =
        context.require_capability_for_cstr(optional_cstr(provider.plugin_id), CAP_FILES_VIRTUALIZE)
    {
        return code;
    }
    if provider.patch_read.is_some() {
        if let Err(code) =
            context.require_capability_for_cstr(optional_cstr(provider.plugin_id), CAP_RDB_PATCH)
        {
            return code;
        }
    }
    super::register_file_provider(provider, optional_cstr(provider.plugin_id))
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use plugin_abi::Oppw4FileProvider;

    use super::*;
    use crate::runtime::ffi::ApiContext;

    unsafe extern "system" fn open_path(
        _provider_context: *mut c_void,
        _path_utf8: *const std::ffi::c_char,
        _out_handle: *mut u64,
    ) -> i32 {
        0
    }

    unsafe extern "system" fn read(
        _provider_context: *mut c_void,
        _handle: u64,
        _buffer: *mut u8,
        _bytes_to_read: u32,
        _requested_offset: i64,
        _out_bytes_read: *mut u32,
    ) -> i32 {
        0
    }

    unsafe extern "system" fn close(_provider_context: *mut c_void, _handle: u64) -> i32 {
        1
    }

    unsafe extern "system" fn size(
        _provider_context: *mut c_void,
        _handle: u64,
        _out_size: *mut u64,
    ) -> i32 {
        0
    }

    unsafe extern "system" fn seek(
        _provider_context: *mut c_void,
        _handle: u64,
        _distance: i64,
        _move_method: u32,
        _out_position: *mut u64,
    ) -> i32 {
        0
    }

    unsafe extern "system" fn patch_read(
        _provider_context: *mut c_void,
        _path_utf8: *const std::ffi::c_char,
        _os_handle: usize,
        _read_offset: u64,
        _buffer: *mut u8,
        _len: usize,
    ) -> i32 {
        0
    }

    #[test]
    fn patch_read_provider_requires_rdb_patch_capability() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            [CAP_FILES_VIRTUALIZE.to_string()],
            Vec::<String>::new(),
        );
        let provider = file_provider(Some(patch_read));

        let code = unsafe {
            host_register_file_provider(
                (&context as *const ApiContext).cast_mut().cast(),
                &provider,
            )
        };

        assert_eq!(code, -22);
    }

    fn file_provider(
        patch_read: Option<plugin_abi::Oppw4ProviderPatchReadFn>,
    ) -> Oppw4FileProvider {
        Oppw4FileProvider {
            plugin_id: c"skin_patcher".as_ptr(),
            provider_context: std::ptr::null_mut(),
            open_path: Some(open_path),
            read: Some(read),
            close: Some(close),
            size: Some(size),
            file_time: None,
            seek: Some(seek),
            patch_read,
        }
    }
}
