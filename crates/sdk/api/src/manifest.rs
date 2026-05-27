use std::path::{Path, PathBuf};

pub const PLUGIN_MANIFEST_FILE: &str = "plugin.toml";
pub const MOD_MANIFEST_FILE: &str = "mod.toml";
pub const PLUGIN_LOGS_DIR: &str = "logs";
pub const PLUGIN_MODS_DIR: &str = "mods";
pub const DEFAULT_PLUGIN_VERSION: &str = "0.2.0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginDescriptor {
    pub id: String,
    pub version: String,
    pub entry: String,
    pub dependencies: Vec<String>,
    pub registry_modules: Vec<RegistryModuleDescriptor>,
    pub capabilities_required: Vec<String>,
    pub capabilities_provided: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryModuleDescriptor {
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub uses_plugins: Vec<String>,
    pub entry: ModEntry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModEntry {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginManifestError {
    InvalidToml,
    MissingPluginTable,
    MissingModTable,
    MissingId,
    MissingName,
    MissingVersion,
    MissingEntry,
    InvalidEntry(String),
    InvalidModEntry(String),
    InvalidRegistryModule(String),
    InvalidCapability(String),
}

impl PluginDescriptor {
    pub fn parse_toml(text: &str) -> Result<Self, PluginManifestError> {
        let value = text
            .parse::<toml::Value>()
            .map_err(|_| PluginManifestError::InvalidToml)?;
        let plugin = value
            .get("plugin")
            .and_then(toml::Value::as_table)
            .ok_or(PluginManifestError::MissingPluginTable)?;
        let id = plugin
            .get("id")
            .and_then(toml::Value::as_str)
            .map(sanitize_plugin_id)
            .filter(|id| id != "unknown_plugin")
            .ok_or(PluginManifestError::MissingId)?;
        let version = plugin
            .get("version")
            .and_then(toml::Value::as_str)
            .filter(|version| !version.trim().is_empty())
            .ok_or(PluginManifestError::MissingVersion)?
            .to_string();
        let entry = plugin
            .get("entry")
            .and_then(toml::Value::as_str)
            .ok_or(PluginManifestError::MissingEntry)?;
        let entry = plugin_entry_file_name(entry)?;
        let dependencies = string_array(&value, &["dependencies", "plugins"])
            .into_iter()
            .map(|dependency| sanitize_plugin_id(&dependency))
            .filter(|id| id != "unknown_plugin")
            .collect::<Vec<_>>();
        let dependencies = unique_strings(dependencies);
        let registry_modules = registry_module_descriptors(&value)?;
        let capabilities_required = string_array(&value, &["capabilities", "requires"])
            .into_iter()
            .map(normalize_capability_name)
            .collect::<Result<Vec<_>, _>>()?;
        let capabilities_required = unique_strings(capabilities_required);
        let capabilities_provided = string_array(&value, &["capabilities", "provides"])
            .into_iter()
            .map(normalize_capability_name)
            .collect::<Result<Vec<_>, _>>()?;
        let capabilities_provided = unique_strings(capabilities_provided);

        Ok(Self {
            id,
            version,
            entry,
            dependencies,
            registry_modules,
            capabilities_required,
            capabilities_provided,
        })
    }

    pub fn default_for_folder(folder_name: &str, entry: &str) -> Result<Self, PluginManifestError> {
        let id = sanitize_plugin_id(folder_name);
        if id == "unknown_plugin" {
            return Err(PluginManifestError::MissingId);
        }
        Ok(Self {
            id,
            version: DEFAULT_PLUGIN_VERSION.to_string(),
            entry: plugin_entry_file_name(entry)?,
            dependencies: Vec::new(),
            registry_modules: Vec::new(),
            capabilities_required: Vec::new(),
            capabilities_provided: Vec::new(),
        })
    }

    pub fn to_toml(&self) -> String {
        format!(
            "[plugin]\nid = \"{}\"\nversion = \"{}\"\nentry = \"{}\"\n",
            escape_toml_string(&self.id),
            escape_toml_string(&self.version),
            escape_toml_string(&self.entry)
        )
    }
}

impl RegistryModuleDescriptor {
    pub fn new(module: impl AsRef<str>) -> Result<Self, PluginManifestError> {
        Ok(Self {
            module: normalize_registry_module_name(module.as_ref())?,
        })
    }
}

impl ModManifest {
    pub fn parse_toml(text: &str) -> Result<Self, PluginManifestError> {
        let value = text
            .parse::<toml::Value>()
            .map_err(|_| PluginManifestError::InvalidToml)?;
        let module = value
            .get("mod")
            .and_then(toml::Value::as_table)
            .ok_or(PluginManifestError::MissingModTable)?;
        let id = module
            .get("id")
            .and_then(toml::Value::as_str)
            .map(sanitize_plugin_id)
            .filter(|id| id != "unknown_plugin")
            .ok_or(PluginManifestError::MissingId)?;
        let name = module
            .get("name")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or(PluginManifestError::MissingName)?
            .to_string();
        let uses_plugins = string_array(&value, &["uses", "plugins"])
            .into_iter()
            .map(|plugin| sanitize_plugin_id(&plugin))
            .filter(|id| id != "unknown_plugin")
            .collect::<Vec<_>>();
        let uses_plugins = unique_strings(uses_plugins);
        let entry = ModEntry::parse(&value)?;

        Ok(Self {
            id,
            name,
            uses_plugins,
            entry,
        })
    }
}

impl ModEntry {
    fn parse(value: &toml::Value) -> Result<Self, PluginManifestError> {
        let entry = value
            .get("entry")
            .and_then(toml::Value::as_table)
            .ok_or(PluginManifestError::MissingEntry)?;
        let path = entry
            .get("file")
            .or_else(|| entry.get("main"))
            .and_then(toml::Value::as_str)
            .ok_or(PluginManifestError::MissingEntry)?;
        Self::new(path)
    }

    pub fn new(path: impl AsRef<str>) -> Result<Self, PluginManifestError> {
        Ok(Self {
            path: mod_entry_file_name(path.as_ref())?,
        })
    }
}

pub fn parse_mod_manifest(text: &str) -> Result<ModManifest, PluginManifestError> {
    ModManifest::parse_toml(text)
}

fn string_array(value: &toml::Value, path: &[&str]) -> Vec<String> {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_registry_module_name(raw: &str) -> Result<String, PluginManifestError> {
    let name = raw.trim().to_ascii_lowercase();
    if is_dotted_ascii_name(&name) {
        Ok(name)
    } else {
        Err(PluginManifestError::InvalidRegistryModule(raw.to_string()))
    }
}

fn normalize_capability_name(raw: String) -> Result<String, PluginManifestError> {
    let name = raw.trim().to_ascii_lowercase();
    if is_dotted_ascii_name(&name) {
        Ok(name)
    } else {
        Err(PluginManifestError::InvalidCapability(raw))
    }
}

fn is_dotted_ascii_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains("..")
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
}

fn registry_module_descriptors(
    value: &toml::Value,
) -> Result<Vec<RegistryModuleDescriptor>, PluginManifestError> {
    let mut modules = Vec::new();
    if let Some(items) = value
        .get("registry")
        .and_then(|registry| registry.get("modules"))
        .and_then(toml::Value::as_array)
    {
        for item in items {
            if let Some(module) = item.as_str() {
                modules.push(RegistryModuleDescriptor::new(module)?);
                continue;
            }
            if let Some(module) = item
                .as_table()
                .and_then(|table| table.get("module").or_else(|| table.get("name")))
                .and_then(toml::Value::as_str)
            {
                modules.push(RegistryModuleDescriptor::new(module)?);
            }
        }
    }
    Ok(unique_registry_modules(modules))
}

fn unique_registry_modules(values: Vec<RegistryModuleDescriptor>) -> Vec<RegistryModuleDescriptor> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique.iter().any(|known: &RegistryModuleDescriptor| {
            known.module.eq_ignore_ascii_case(&value.module)
        }) {
            unique.push(value);
        }
    }
    unique
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique
            .iter()
            .any(|known: &String| known.eq_ignore_ascii_case(&value))
        {
            unique.push(value);
        }
    }
    unique
}

pub fn plugin_toml_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(PLUGIN_MANIFEST_FILE)
}

pub fn plugin_logs_root(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(PLUGIN_LOGS_DIR)
}

pub fn plugin_mods_root(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(PLUGIN_MODS_DIR)
}

pub fn sanitize_plugin_id(raw: &str) -> String {
    let sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown_plugin".to_string()
    } else {
        sanitized
    }
}

pub fn plugin_entry_file_name(raw: &str) -> Result<String, PluginManifestError> {
    let path = Path::new(raw);
    if path.is_absolute()
        || path.components().count() != 1
        || path.file_name().and_then(|value| value.to_str()) != Some(raw)
    {
        return Err(PluginManifestError::InvalidEntry(raw.to_string()));
    }
    Ok(raw.to_string())
}

pub fn mod_entry_file_name(raw: &str) -> Result<String, PluginManifestError> {
    let normalized = raw.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute()
        || normalized.starts_with('/')
        || normalized.contains("../")
        || normalized == ".."
        || normalized.starts_with("..")
        || normalized.split('/').any(str::is_empty)
    {
        return Err(PluginManifestError::InvalidModEntry(raw.to_string()));
    }
    Ok(normalized)
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_toml_descriptor() {
        let descriptor = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "example_plugin"
                version = "0.2.0"
                entry = "example_plugin.dll"

                [dependencies]
                plugins = ["sdk_runtime"]

                [registry]
                modules = ["example_plugin"]

                [capabilities]
                requires = ["registry.module", "hooks.install"]
                provides = ["game.characters.extend"]
            "#,
        )
        .expect("descriptor");

        assert_eq!(
            descriptor,
            PluginDescriptor {
                id: "example_plugin".to_string(),
                version: "0.2.0".to_string(),
                entry: "example_plugin.dll".to_string(),
                dependencies: vec!["sdk_runtime".to_string()],
                registry_modules: vec![RegistryModuleDescriptor {
                    module: "example_plugin".to_string(),
                }],
                capabilities_required: vec![
                    "registry.module".to_string(),
                    "hooks.install".to_string()
                ],
                capabilities_provided: vec!["game.characters.extend".to_string()],
            }
        );
    }

    #[test]
    fn rejects_nested_entry_paths() {
        let error = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "bad"
                version = "0.2.0"
                entry = "bin/bad.dll"
            "#,
        )
        .expect_err("entry path");

        assert_eq!(
            error,
            PluginManifestError::InvalidEntry("bin/bad.dll".to_string())
        );
    }

    #[test]
    fn rejects_invalid_registry_module_names() {
        let error = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "bad"
                version = "0.2.0"
                entry = "bad.dll"

                [registry]
                modules = ["../bad"]
            "#,
        )
        .expect_err("bad registry module");

        assert_eq!(
            error,
            PluginManifestError::InvalidRegistryModule("../bad".to_string())
        );
    }

    #[test]
    fn rejects_invalid_capability_names() {
        let error = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "bad"
                version = "0.2.0"
                entry = "bad.dll"

                [capabilities]
                requires = ["../memory.write"]
            "#,
        )
        .expect_err("bad capability");

        assert_eq!(
            error,
            PluginManifestError::InvalidCapability("../memory.write".to_string())
        );
    }

    #[test]
    fn normalizes_capability_names() {
        let descriptor = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "example_plugin"
                version = "0.2.0"
                entry = "example_plugin.dll"

                [capabilities]
                requires = [" Registry.Module ", "HOOKS.INSTALL"]
                provides = [" Runtime.Fx "]
            "#,
        )
        .expect("descriptor");

        assert_eq!(
            descriptor.capabilities_required,
            ["registry.module", "hooks.install"]
        );
        assert_eq!(descriptor.capabilities_provided, ["runtime.fx"]);
    }

    #[test]
    fn deduplicates_normalized_manifest_lists() {
        let descriptor = PluginDescriptor::parse_toml(
            r#"
                [plugin]
                id = "example_plugin"
                version = "0.2.0"
                entry = "example_plugin.dll"

                [dependencies]
                plugins = ["SDK Runtime", "sdk_runtime"]

                [registry]
                modules = ["Example_Plugin", "example_plugin"]

                [capabilities]
                requires = [" Registry.Module ", "registry.module", "Memory.Scan"]
                provides = [" Runtime.Fx ", "runtime.fx"]
            "#,
        )
        .expect("descriptor");

        assert_eq!(descriptor.dependencies, ["SDK_Runtime"]);
        assert_eq!(
            descriptor.registry_modules,
            [RegistryModuleDescriptor {
                module: "example_plugin".to_string(),
            }]
        );
        assert_eq!(
            descriptor.capabilities_required,
            ["registry.module", "memory.scan"]
        );
        assert_eq!(descriptor.capabilities_provided, ["runtime.fx"]);
    }

    #[test]
    fn default_manifest_uses_folder_name_and_entry() {
        let descriptor =
            PluginDescriptor::default_for_folder("example plugin", "example_plugin.dll").unwrap();

        assert_eq!(descriptor.id, "example_plugin");
        assert_eq!(descriptor.version, DEFAULT_PLUGIN_VERSION);
        assert_eq!(
            descriptor.to_toml(),
            "[plugin]\nid = \"example_plugin\"\nversion = \"0.2.0\"\nentry = \"example_plugin.dll\"\n"
        );
    }

    #[test]
    fn parses_mod_manifest_entry() {
        let manifest = parse_mod_manifest(
            r#"
                [mod]
                id = "test_mod"
                name = "Test Mod"

                [uses]
                plugins = ["sdk runtime", "sdk_runtime"]

                [entry]
                file = "main.mod"
            "#,
        )
        .expect("mod manifest");

        assert_eq!(
            manifest,
            ModManifest {
                id: "test_mod".to_string(),
                name: "Test Mod".to_string(),
                uses_plugins: vec!["sdk_runtime".to_string()],
                entry: ModEntry {
                    path: "main.mod".to_string(),
                },
            }
        );
    }

    #[test]
    fn rejects_unsafe_mod_entry_paths() {
        let error = parse_mod_manifest(
            r#"
                [mod]
                id = "bad"
                name = "Bad"

                [entry]
                file = "../main.mod"
            "#,
        )
        .expect_err("unsafe entry");

        assert_eq!(
            error,
            PluginManifestError::InvalidModEntry("../main.mod".to_string())
        );
    }
}
