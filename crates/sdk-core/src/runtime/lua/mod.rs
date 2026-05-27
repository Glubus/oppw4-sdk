mod bridge;
mod hot_reload;
mod logs;
mod module;
mod owned_modules;
mod runner;
mod state;

use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use plugin_abi::{optional_cstr, Oppw4LuaModule};

use crate::log;

use self::{hot_reload::start_hot_reload_worker, module::RegisteredModule, state::LuaHost};

pub(crate) use module::ModulePermissions;

static HOST: OnceLock<Mutex<LuaHost>> = OnceLock::new();

pub(crate) fn initialize(mods_root: &Path) {
    let host = HOST.get_or_init(|| Mutex::new(LuaHost::default()));
    let mut host = host.lock().expect("lua host lock");
    host.reset(mods_root);
    if host.start_hot_reload() {
        start_hot_reload_worker(mods_root.to_path_buf());
    }
}

pub(crate) unsafe fn register_module(
    module: *const Oppw4LuaModule,
    permissions: ModulePermissions,
) -> i32 {
    let Some(module) = module.as_ref() else {
        return -1;
    };
    let Some(plugin_id) = optional_cstr(module.plugin_id) else {
        return -2;
    };
    let Some(module_name) = optional_cstr(module.module_name) else {
        return -3;
    };
    let Some(register) = module.register else {
        return -4;
    };
    let Some(host) = HOST.get() else {
        return -5;
    };
    let mut host = host.lock().expect("lua host lock");
    let entry = RegisteredModule {
        plugin_id: plugin_id.to_string_lossy().into_owned(),
        module_name: module_name.to_string_lossy().into_owned(),
        context: module.module_context as usize,
        register,
        permissions,
    };
    if let Err(error) = host.register_module(entry.clone()) {
        log::write_line(format!(
            "lua host: rejected module plugin={} module={} error={error}",
            entry.plugin_id, entry.module_name
        ));
        return -6;
    }
    log::write_line(format!(
        "lua host: registered module plugin={} module={}",
        entry.plugin_id, entry.module_name
    ));
    0
}

pub(crate) fn run_ready_mods() {
    with_host(|host| host.run_ready_mods());
}

fn with_host(action: impl FnOnce(&mut LuaHost)) {
    let Some(host) = HOST.get() else {
        return;
    };
    action(&mut host.lock().expect("lua host lock"));
}
