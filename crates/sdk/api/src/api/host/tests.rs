use std::{
    ffi::{c_char, c_void, CStr},
    sync::{Mutex, OnceLock},
};

use plugin_abi::{
    cstring_lossy, null_api, HostPluginModZipVisitorFn, Oppw4LogEntry, Oppw4PluginApi,
};

use super::{HostApi, OwnedHostApi};

static CAPTURED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

unsafe extern "system" fn capture_log(_host_context: *mut c_void, entry: *const Oppw4LogEntry) {
    let entry = &*entry;
    let plugin_id = CStr::from_ptr(entry.plugin_id).to_string_lossy();
    let message = CStr::from_ptr(entry.message).to_string_lossy();
    CAPTURED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("capture lock")
        .push(format!("{plugin_id}:{message}"));
}

unsafe extern "system" fn visit_mod_zips(
    _host_context: *mut c_void,
    visitor: Option<HostPluginModZipVisitorFn>,
    user_context: *mut c_void,
) -> i32 {
    let Some(visitor) = visitor else {
        return -1;
    };
    let a = cstring_lossy(r"D:\Game\OPPW4\plugins\skin_patcher\mods\a.zip");
    let b = cstring_lossy(r"D:\Game\OPPW4\plugins\skin_patcher\mods\nested\b.zip");
    if visitor(user_context, a.as_ptr()) != 0 {
        return -2;
    }
    visitor(user_context, b.as_ptr())
}

unsafe extern "system" fn debug_enabled(_host_context: *mut c_void) -> i32 {
    1
}

unsafe extern "system" fn require_capability(
    _host_context: *mut c_void,
    plugin_id: *const c_char,
    capability: *const c_char,
) -> i32 {
    let plugin_id = CStr::from_ptr(plugin_id).to_string_lossy();
    let capability = CStr::from_ptr(capability).to_string_lossy();
    if plugin_id == "fx_director" && capability == "hooks.install" {
        0
    } else {
        -22
    }
}

#[test]
fn api_keeps_access_to_raw_abi_when_needed() {
    let abi = null_api();
    let host = HostApi::from(&abi);

    assert_eq!(host.abi().version, abi.version);
}

#[test]
fn owned_api_can_be_cloned_into_worker_state() {
    let owned = OwnedHostApi::from(Oppw4PluginApi {
        version: 42,
        ..null_api()
    });
    let worker_handle = owned.clone();

    assert_eq!(worker_handle.abi().version, 42);
    assert_eq!(owned.as_ref().abi().version, 42);
}

#[test]
fn host_log_service_forwards_plugin_id_and_message() {
    CAPTURED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("capture lock")
        .clear();
    let api = Oppw4PluginApi {
        log: Some(capture_log),
        ..null_api()
    };

    HostApi::from(&api)
        .log()
        .write("skin_patcher", "hello")
        .expect("log write");

    assert_eq!(
        CAPTURED.get().unwrap().lock().unwrap().as_slice(),
        ["skin_patcher:hello"]
    );
}

#[test]
fn host_mod_service_collects_legacy_paths() {
    let api = Oppw4PluginApi {
        for_each_plugin_mod_zip: Some(visit_mod_zips),
        ..null_api()
    };

    assert_eq!(
        HostApi::from(&api).mods().legacy_paths(),
        [
            r"D:\Game\OPPW4\plugins\skin_patcher\mods\a.zip",
            r"D:\Game\OPPW4\plugins\skin_patcher\mods\nested\b.zip",
        ]
    );
}

#[test]
fn host_game_service_reports_debug_flag() {
    assert!(!HostApi::from(&null_api()).game().debug_enabled());
    let api = Oppw4PluginApi {
        debug_enabled: Some(debug_enabled),
        ..null_api()
    };

    assert!(HostApi::from(&api).game().debug_enabled());
}

#[test]
fn host_hook_service_requires_install_capability() {
    let api = Oppw4PluginApi {
        require_capability: Some(require_capability),
        ..null_api()
    };

    assert_eq!(
        HostApi::from(&api).hooks().require_install("fx_director"),
        Ok(())
    );

    let error = HostApi::from(&api)
        .hooks()
        .require_install("skin_patcher")
        .expect_err("missing capability should be rejected");
    assert_eq!(
        error,
        crate::PluginError::HostCallFailed {
            operation: "require_capability",
            code: -22
        }
    );
}
