use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = ".sdkt";
const CONFIG_FILE: &str = "config.toml";
const DEFAULT_BRIDGE: &str = "bridge-js";
const DEFAULT_PROFILE: &str = "default";
const DEFAULT_MODS_PATH: &str = ".sdkt/mods";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct SdktConfig {
    pub game_path: Option<PathBuf>,
    pub mods_path: Option<PathBuf>,
    pub profile: Option<String>,
    pub default_bridge: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ConfigPatch {
    pub game_path: Option<PathBuf>,
    pub mods_path: Option<PathBuf>,
    pub profile: Option<String>,
    pub default_bridge: Option<String>,
}

impl ConfigPatch {
    pub(crate) fn is_empty(&self) -> bool {
        self.game_path.is_none()
            && self.mods_path.is_none()
            && self.profile.is_none()
            && self.default_bridge.is_none()
    }
}

pub(crate) fn default_bridge() -> String {
    DEFAULT_BRIDGE.to_string()
}

pub(crate) fn config_dir(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR)
}

pub(crate) fn config_path(root: &Path) -> PathBuf {
    config_dir(root).join(CONFIG_FILE)
}

pub(crate) fn load(root: &Path) -> Result<SdktConfig, String> {
    let path = config_path(root);
    if !path.exists() {
        return Ok(default_config());
    }
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn save(root: &Path, config: &SdktConfig) -> Result<PathBuf, String> {
    let dir = config_dir(root);
    fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let path = config_path(root);
    let text = toml::to_string_pretty(config)
        .map_err(|error| format!("failed to serialize config: {error}"))?;
    fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(path)
}

pub(crate) fn update(root: &Path, patch: ConfigPatch) -> Result<SdktConfig, String> {
    let mut config = load(root)?;
    if let Some(game_path) = patch.game_path {
        config.game_path = Some(game_path);
    }
    if let Some(mods_path) = patch.mods_path {
        config.mods_path = Some(mods_path);
    }
    if let Some(profile) = patch.profile {
        config.profile = Some(profile);
    }
    if let Some(default_bridge) = patch.default_bridge {
        config.default_bridge = Some(default_bridge);
    }
    save(root, &config)?;
    Ok(config)
}

pub(crate) fn format(config: &SdktConfig) -> Result<String, String> {
    toml::to_string_pretty(config).map_err(|error| format!("failed to serialize config: {error}"))
}

pub(crate) fn default_config() -> SdktConfig {
    SdktConfig {
        game_path: None,
        mods_path: Some(PathBuf::from(DEFAULT_MODS_PATH)),
        profile: Some(DEFAULT_PROFILE.to_string()),
        default_bridge: Some(DEFAULT_BRIDGE.to_string()),
    }
}

pub(crate) fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
