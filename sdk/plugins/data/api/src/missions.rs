use std::{fs, path::Path, sync::OnceLock};

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Mission {
    pub id: String,
    pub display_name: Option<String>,
    pub aliases: Vec<String>,
    pub mission_id: Option<u16>,
    pub linkdata_id: Option<u16>,
    pub modes: Vec<String>,
    pub difficulties: Option<MissionDifficulties>,
    pub rank_conditions: Option<MissionRankConditions>,
    pub rewards: Option<MissionRewards>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MissionDifficulties {
    #[serde(default)]
    pub observations: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MissionRankConditions {
    #[serde(default)]
    pub observations: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MissionRewards {
    #[serde(default)]
    pub observations: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct MissionIndexFile {
    #[serde(default)]
    missions: Vec<MissionIndexEntry>,
}

#[derive(Debug, Deserialize)]
struct MissionIndexEntry {
    path: String,
    difficulties: Option<String>,
    rank_conditions: Option<String>,
    rewards: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MissionDataFile {
    id: String,
    display_name: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    ids: MissionIds,
    #[serde(default)]
    modes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MissionIds {
    mission: Option<u16>,
    linkdata: Option<u16>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MissionDataError {
    InvalidJson(String),
    InvalidDirectory(String),
}

static MISSIONS: OnceLock<Vec<Mission>> = OnceLock::new();

pub fn all() -> &'static [Mission] {
    MISSIONS.get_or_init(Vec::new).as_slice()
}

pub fn initialize_data_root(root: &Path) -> Result<(), MissionDataError> {
    let missions = read_data_root(root)?;
    let _ = MISSIONS.set(missions);
    Ok(())
}

pub fn mark_data_unavailable() {
    let _ = MISSIONS.set(Vec::new());
}

pub fn find(query: &str) -> Option<&'static Mission> {
    let query = normalize(query);
    if query.is_empty() {
        return None;
    }

    all().iter().find(|mission| {
        normalize(&mission.id) == query
            || mission
                .display_name
                .as_deref()
                .is_some_and(|name| normalize(name) == query)
            || mission
                .aliases
                .iter()
                .any(|alias| normalize(alias) == query)
    })
}

pub fn find_by_id(id: u16) -> Option<&'static Mission> {
    all().iter().find(|mission| mission.mission_id == Some(id))
}

pub fn read_data_root(root: &Path) -> Result<Vec<Mission>, MissionDataError> {
    let index_path = root.join("generated").join("index.json");
    if index_path.is_file() {
        return read_indexed_data_root(root, &index_path);
    }

    let missions_root = root.join("missions");
    if !missions_root.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&missions_root)
        .map_err(|error| MissionDataError::InvalidDirectory(error.to_string()))?;
    let mut missions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let data_path = path.join("data.json");
        if data_path.is_file() {
            missions.push(read_mission_dir(None, &path, &data_path, None, None, None)?);
        }
    }
    missions.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(missions)
}

fn read_indexed_data_root(
    root: &Path,
    index_path: &Path,
) -> Result<Vec<Mission>, MissionDataError> {
    let text = fs::read_to_string(index_path)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))?;
    let index: MissionIndexFile = serde_json::from_str(&text)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))?;
    let mut missions = Vec::with_capacity(index.missions.len());
    for entry in index.missions {
        let data_path = root.join(&entry.path);
        let mission_dir = data_path
            .parent()
            .ok_or_else(|| MissionDataError::InvalidDirectory(entry.path.clone()))?;
        missions.push(read_mission_dir(
            Some(root),
            mission_dir,
            &data_path,
            entry.difficulties.as_deref(),
            entry.rank_conditions.as_deref(),
            entry.rewards.as_deref(),
        )?);
    }
    missions.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(missions)
}

fn read_mission_dir(
    root: Option<&Path>,
    mission_dir: &Path,
    data_path: &Path,
    difficulties_ref: Option<&str>,
    rank_conditions_ref: Option<&str>,
    rewards_ref: Option<&str>,
) -> Result<Mission, MissionDataError> {
    let text = fs::read_to_string(data_path)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))?;
    let data: MissionDataFile = serde_json::from_str(&text)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))?;
    Ok(Mission {
        id: data.id,
        display_name: data.display_name,
        aliases: data.aliases,
        mission_id: data.ids.mission,
        linkdata_id: data.ids.linkdata,
        modes: data.modes,
        difficulties: read_optional_json(root, mission_dir, difficulties_ref, "difficulties.json")?,
        rank_conditions: read_optional_json(
            root,
            mission_dir,
            rank_conditions_ref,
            "rank_conditions.json",
        )?,
        rewards: read_optional_json(root, mission_dir, rewards_ref, "rewards.json")?,
    })
}

fn read_optional_json<T: for<'de> Deserialize<'de>>(
    root: Option<&Path>,
    mission_dir: &Path,
    indexed_ref: Option<&str>,
    fallback: &str,
) -> Result<Option<T>, MissionDataError> {
    let path = indexed_ref.map_or_else(
        || mission_dir.join(fallback),
        |path| root.unwrap_or(mission_dir).join(path),
    );
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| MissionDataError::InvalidJson(error.to_string()))
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_indexed_missions_from_workspace_data() {
        let root = workspace_data_root();
        let missions = read_data_root(&root).expect("missions");
        let mission = missions
            .iter()
            .find(|mission| mission.mission_id == Some(77))
            .expect("mission 77");

        assert_eq!(mission.id, "mission_0077");
        assert!(mission.rank_conditions.is_some());
        assert!(mission.rewards.is_some());
    }

    #[test]
    fn find_by_runtime_mission_id_works_after_initialize() {
        let root = workspace_data_root();
        initialize_data_root(&root).expect("mission root");
        assert_eq!(
            find_by_id(35).map(|mission| mission.id.as_str()),
            Some("mission_0035")
        );
    }

    fn workspace_data_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("..")
            .join("oppw4-data")
    }
}
