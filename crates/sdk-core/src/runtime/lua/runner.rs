use std::{fs, path::Path};

use crate::runtime::logs as host_logs;
use crate::{log, runtime::linkdata};

use super::logs::write_mod_entries;
use super::module::{register_plugin_module, RegisteredModule};
use super::owned_modules;

#[derive(Clone, Copy)]
pub(super) enum ModRunReason {
    Initial,
    HotReload,
}

impl ModRunReason {
    fn success_label(self) -> &'static str {
        match self {
            Self::Initial => "applied",
            Self::HotReload => "hot-reloaded",
        }
    }

    fn failure_label(self) -> &'static str {
        match self {
            Self::Initial => "pending/failed",
            Self::HotReload => "hot-reload failed",
        }
    }
}

pub(super) fn run_mod(
    mod_entry: &lua_api::LuaMod,
    modules: Vec<RegisteredModule>,
    reason: ModRunReason,
) -> bool {
    host_logs::write_mod(
        "lua_host",
        &format!(
            "mod start id={} uses={:?} modules={}",
            mod_entry.manifest.id,
            mod_entry.manifest.uses_plugins,
            modules
                .iter()
                .map(|module| format!("{}:{}", module.plugin_id, module.module_name))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    let result = lua_api::run_lua_mod(mod_entry, |lua| {
        owned_modules::install(lua)?;
        let mod_id = mod_entry.manifest.id.clone();
        lua.globals().set(
            "__oppw4_trace",
            lua.create_function(move |_, message: String| {
                host_logs::write_mod("lua_host", &format!("trace mod={mod_id} {message}"));
                Ok(())
            })?,
        )?;
        for module in modules {
            if is_sdk_owned_module(&module) {
                host_logs::write_mod(
                    "lua_host",
                    &format!(
                        "module register skipped sdk-owned mod={} plugin={} module={}",
                        mod_entry.manifest.id, module.plugin_id, module.module_name
                    ),
                );
                continue;
            }
            host_logs::write_mod(
                "lua_host",
                &format!(
                    "module register start mod={} plugin={} module={}",
                    mod_entry.manifest.id, module.plugin_id, module.module_name
                ),
            );
            register_plugin_module(lua, &module)?;
            host_logs::write_mod(
                "lua_host",
                &format!(
                    "module register ok mod={} plugin={} module={}",
                    mod_entry.manifest.id, module.plugin_id, module.module_name
                ),
            );
        }
        host_logs::write_mod(
            "lua_host",
            &format!("mod body start id={}", mod_entry.manifest.id),
        );
        Ok(())
    });
    match result {
        Ok(report) => {
            write_mod_entries(mod_entry, &report.logs);
            if let Err(error) = apply_boot_mutations(mod_entry, &report.mutations) {
                let message = format!(
                    "lua host: mod {} id={} mutation error={error}",
                    reason.failure_label(),
                    mod_entry.manifest.id
                );
                log::write_line(&message);
                host_logs::write_mod("lua_host", &message);
                return false;
            }
            host_logs::write_mod(
                "lua_host",
                &format!("mod body ok id={}", mod_entry.manifest.id),
            );
            log::write_line(format!(
                "lua host: mod {} id={} uses={:?}",
                reason.success_label(),
                mod_entry.manifest.id,
                mod_entry.manifest.uses_plugins
            ));
            true
        }
        Err(error) => {
            let message = format!(
                "lua host: mod {} id={} error={error:?}",
                reason.failure_label(),
                mod_entry.manifest.id
            );
            log::write_line(&message);
            host_logs::write_mod("lua_host", &message);
            false
        }
    }
}

pub(super) fn run_initial_mods(
    mods: Vec<(lua_api::LuaMod, Vec<RegisteredModule>)>,
) -> Vec<(lua_api::LuaMod, bool)> {
    if mods.is_empty() {
        return Vec::new();
    }

    let all_mods = mods
        .iter()
        .map(|(mod_entry, _)| mod_entry.clone())
        .collect::<Vec<_>>();
    let modules = unique_modules(mods.iter().flat_map(|(_, modules)| modules.iter().cloned()));
    host_logs::write_mod(
        "lua_host",
        &format!(
            "batch start mods={} modules={}",
            all_mods.len(),
            modules
                .iter()
                .map(|module| format!("{}:{}", module.plugin_id, module.module_name))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );

    let reports = lua_api::run_lua_mods(
        &all_mods,
        |lua| {
            owned_modules::install(lua)?;
            lua.globals().set(
                "__oppw4_trace",
                lua.create_function(move |lua, message: String| {
                    let mod_id = lua
                        .globals()
                        .get::<Option<String>>("__oppw4_mod_id")?
                        .unwrap_or_else(|| "unknown".to_string());
                    host_logs::write_mod("lua_host", &format!("trace mod={mod_id} {message}"));
                    Ok(())
                })?,
            )?;
            for module in modules {
                if is_sdk_owned_module(&module) {
                    host_logs::write_mod(
                        "lua_host",
                        &format!(
                            "batch module register skipped sdk-owned plugin={} module={}",
                            module.plugin_id, module.module_name
                        ),
                    );
                    continue;
                }
                host_logs::write_mod(
                    "lua_host",
                    &format!(
                        "batch module register start plugin={} module={}",
                        module.plugin_id, module.module_name
                    ),
                );
                register_plugin_module(lua, &module)?;
                host_logs::write_mod(
                    "lua_host",
                    &format!(
                        "batch module register ok plugin={} module={}",
                        module.plugin_id, module.module_name
                    ),
                );
            }
            Ok(())
        },
        |_, mod_entry| {
            host_logs::write_mod(
                "lua_host",
                &format!("batch mod body start id={}", mod_entry.manifest.id),
            );
            Ok(())
        },
    );

    let reports = match reports {
        Ok(reports) => reports,
        Err(error) => {
            let message = format!("lua host: batch setup failed error={error:?}");
            log::write_line(&message);
            host_logs::write_mod("lua_host", &message);
            return all_mods
                .into_iter()
                .map(|mod_entry| (mod_entry, false))
                .collect();
        }
    };

    all_mods
        .into_iter()
        .zip(reports)
        .map(|(mod_entry, report)| match report.result {
            Ok(report) => {
                write_mod_entries(&mod_entry, &report.logs);
                let applied = match apply_boot_mutations(&mod_entry, &report.mutations) {
                    Ok(()) => true,
                    Err(error) => {
                        let message = format!(
                            "lua host: mod {} id={} mutation error={error}",
                            ModRunReason::Initial.failure_label(),
                            mod_entry.manifest.id
                        );
                        log::write_line(&message);
                        host_logs::write_mod("lua_host", &message);
                        false
                    }
                };
                host_logs::write_mod(
                    "lua_host",
                    &format!("batch mod body ok id={}", mod_entry.manifest.id),
                );
                if applied {
                    log::write_line(format!(
                        "lua host: mod {} id={} uses={:?}",
                        ModRunReason::Initial.success_label(),
                        mod_entry.manifest.id,
                        mod_entry.manifest.uses_plugins
                    ));
                }
                (mod_entry, applied)
            }
            Err(error) => {
                let message = format!(
                    "lua host: mod {} id={} error={error:?}",
                    ModRunReason::Initial.failure_label(),
                    mod_entry.manifest.id
                );
                log::write_line(&message);
                host_logs::write_mod("lua_host", &message);
                (mod_entry, false)
            }
        })
        .collect()
}

fn unique_modules(modules: impl Iterator<Item = RegisteredModule>) -> Vec<RegisteredModule> {
    let mut unique = Vec::new();
    for module in modules {
        if unique.iter().any(|known: &RegisteredModule| {
            known.plugin_id.eq_ignore_ascii_case(&module.plugin_id)
                && known.module_name.eq_ignore_ascii_case(&module.module_name)
        }) {
            continue;
        }
        unique.push(module);
    }
    unique
}

fn is_sdk_owned_module(module: &RegisteredModule) -> bool {
    matches!(
        module.module_name.as_str(),
        "std.character" | "character" | "moveset_patcher"
    )
}

fn apply_boot_mutations(
    mod_entry: &lua_api::LuaMod,
    mutations: &[lua_api::LuaMutation],
) -> Result<(), String> {
    for mutation in mutations {
        match mutation.kind.as_str() {
            "moveset.replace" => apply_moveset_replace(mod_entry, mutation)?,
            other => {
                host_logs::write_mod(
                    "lua_host",
                    &format!(
                        "mutation ignored mod={} type={other}",
                        mod_entry.manifest.id
                    ),
                );
            }
        }
    }
    Ok(())
}

fn apply_moveset_replace(
    mod_entry: &lua_api::LuaMod,
    mutation: &lua_api::LuaMutation,
) -> Result<(), String> {
    let entry = mutation
        .entry
        .ok_or_else(|| "moveset.replace missing entry".to_string())?;
    let payload = match (&mutation.payload, &mutation.payload_file) {
        (Some(payload), _) => payload.clone(),
        (None, Some(payload_file)) => read_mod_payload(mod_entry, payload_file)?,
        (None, None) => return Err("moveset.replace missing payload_file or payload".to_string()),
    };
    linkdata::replace_entry_from_runtime("moveset_patcher", 0, u32::from(entry), &payload)?;
    host_logs::write_mod(
        "lua_host",
        &format!(
            "moveset patch registered mod={} character={} entry={} bytes={}",
            mod_entry.manifest.id,
            mutation.character.as_deref().unwrap_or("unknown"),
            entry,
            payload.len()
        ),
    );
    Ok(())
}

fn read_mod_payload(mod_entry: &lua_api::LuaMod, payload_file: &str) -> Result<Vec<u8>, String> {
    if !is_safe_relative_file(payload_file) {
        return Err(format!("unsafe payload path {payload_file}"));
    }
    match &mod_entry.source {
        lua_api::ModSource::Directory(root) => fs::read(root.join(payload_file.replace('/', "\\")))
            .map_err(|error| {
                format!(
                    "failed to read payload {payload_file} for {}: {error}",
                    mod_entry.manifest.id
                )
            }),
        lua_api::ModSource::Zip { .. } => Err(format!(
            "zip mod payloads are not supported by boot mutation drain yet mod={}",
            mod_entry.manifest.id
        )),
    }
}

fn is_safe_relative_file(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
        && path.file_name().is_some()
}
