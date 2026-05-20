use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::log;
use plugin_sdk::manifest::{
    plugin_logs_root, plugin_mods_root, plugin_toml_path, PluginDescriptor, PluginManifestError,
};

#[derive(Debug, PartialEq)]
pub(crate) struct PluginManifest {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<String>,
    pub(crate) lua_modules: Vec<String>,
    pub(crate) capabilities_required: Vec<String>,
    pub(crate) capabilities_provided: Vec<String>,
    pub(crate) root: PathBuf,
    pub(crate) mods_root: PathBuf,
    pub(crate) entry_path: PathBuf,
    pub(crate) log_root: PathBuf,
}

impl PluginManifest {
    pub(crate) fn sdk_service(
        id: &str,
        entry: &str,
        sdk_root: &Path,
        capabilities_required: &[&str],
        capabilities_provided: &[&str],
    ) -> Self {
        Self {
            id: id.to_string(),
            version: "0.1.0".to_string(),
            dependencies: Vec::new(),
            lua_modules: Vec::new(),
            capabilities_required: capabilities_required
                .iter()
                .map(|capability| (*capability).to_string())
                .collect(),
            capabilities_provided: capabilities_provided
                .iter()
                .map(|capability| (*capability).to_string())
                .collect(),
            root: sdk_root.to_path_buf(),
            mods_root: plugin_mods_root(sdk_root).join(id),
            entry_path: sdk_root.join(entry),
            log_root: plugin_logs_root(sdk_root).join(id),
        }
    }

    pub(crate) fn read_from_dir(plugin_dir: &Path) -> Option<Self> {
        let manifest_path = plugin_toml_path(plugin_dir);
        if !manifest_path.is_file() {
            if let Err(error) = create_default_manifest(plugin_dir, &manifest_path) {
                log::write_line(format!(
                    "plugin host: manifest missing path={} error={error}",
                    manifest_path.display()
                ));
                return None;
            }
        }

        let text = match fs::read_to_string(&manifest_path) {
            Ok(text) => text,
            Err(error) => {
                log::write_line(format!(
                    "plugin host: manifest missing path={} error={error}",
                    manifest_path.display()
                ));
                return None;
            }
        };

        match Self::parse(plugin_dir, &text) {
            Ok(manifest) => Some(manifest),
            Err(error) => {
                log::write_line(format!(
                    "plugin host: manifest invalid path={} error={error}",
                    manifest_path.display()
                ));
                None
            }
        }
    }

    fn parse(plugin_dir: &Path, text: &str) -> Result<Self, String> {
        let descriptor = PluginDescriptor::parse_toml(text).map_err(format_manifest_error)?;

        Ok(Self {
            id: descriptor.id,
            version: descriptor.version,
            dependencies: descriptor.dependencies,
            lua_modules: descriptor.lua_modules,
            capabilities_required: descriptor.capabilities_required,
            capabilities_provided: descriptor.capabilities_provided,
            root: plugin_dir.to_path_buf(),
            mods_root: plugin_mods_root(plugin_dir),
            entry_path: plugin_dir.join(descriptor.entry),
            log_root: plugin_logs_root(plugin_dir),
        })
    }
}

fn create_default_manifest(plugin_dir: &Path, manifest_path: &Path) -> Result<(), String> {
    let entry = infer_entry_file(plugin_dir)?;
    let folder_name = plugin_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "cannot infer plugin id from folder name".to_string())?;
    let descriptor =
        PluginDescriptor::default_for_folder(folder_name, &entry).map_err(format_manifest_error)?;
    fs::write(manifest_path, descriptor.to_toml()).map_err(|error| error.to_string())
}

fn infer_entry_file(plugin_dir: &Path) -> Result<String, String> {
    let folder_name = plugin_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "cannot infer plugin folder name".to_string())?;
    let preferred = plugin_dir.join(format!("{folder_name}.dll"));
    if preferred.is_file() {
        return Ok(preferred
            .file_name()
            .and_then(|name| name.to_str())
            .expect("preferred file name")
            .to_string());
    }

    let mut dlls = fs::read_dir(plugin_dir)
        .map_err(|error| error.to_string())?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_dll = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
            if is_dll {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    dlls.sort();

    match dlls.as_slice() {
        [entry] => Ok(entry.clone()),
        [] => Err("cannot create plugin.toml without a dll in the plugin folder".to_string()),
        _ => Err("cannot create plugin.toml because multiple dll files exist".to_string()),
    }
}

fn format_manifest_error(error: PluginManifestError) -> String {
    match error {
        PluginManifestError::InvalidToml => "invalid TOML".to_string(),
        PluginManifestError::MissingPluginTable => "missing [plugin] table".to_string(),
        PluginManifestError::MissingId => "missing plugin.id".to_string(),
        PluginManifestError::MissingVersion => "missing plugin.version".to_string(),
        PluginManifestError::MissingEntry => "missing plugin.entry".to_string(),
        PluginManifestError::InvalidEntry(entry) => {
            format!("entry must be a file name only: {entry}")
        }
        PluginManifestError::InvalidLuaModule(module) => {
            format!("lua module name is invalid: {module}")
        }
        PluginManifestError::InvalidCapability(capability) => {
            format!("capability name is invalid: {capability}")
        }
    }
}

#[cfg(test)]
mod tests;
