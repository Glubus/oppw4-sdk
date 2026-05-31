use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use serde::Deserialize;

const EMBEDDED_CHARACTERS_JSON: &str = include_str!("../data/characters.json");

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Character {
    pub playable_id: Option<u16>,
    #[serde(default)]
    pub runtime_id: Option<u16>,
    #[serde(default)]
    pub boss_runtime_id: Option<u16>,
    #[serde(default)]
    pub moveset_linkdata_entry: Option<u16>,
    #[serde(default)]
    pub model_id: Option<u16>,
    pub canonical: String,
    pub display_name: String,
    pub model_stem: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub costumes: Vec<CharacterCostume>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CharacterCostume {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub model_id: Option<u16>,
    #[serde(default)]
    pub assets: Vec<CharacterAsset>,
    #[serde(default)]
    pub body_parts: Vec<CharacterBodyPart>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CharacterBodyPart {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub assets: Vec<CharacterAsset>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CharacterAsset {
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub archive: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterDataFile {
    id: String,
    display_name: String,
    #[serde(default)]
    aliases: Vec<String>,
    ids: CharacterIds,
    assets: CharacterAssets,
}

#[derive(Debug, Deserialize)]
struct CharacterIndexFile {
    characters: Vec<CharacterIndexEntry>,
}

#[derive(Debug, Deserialize)]
struct CharacterIndexEntry {
    path: String,
    movesets: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CharacterIds {
    playable: Option<u16>,
    runtime: Option<u16>,
    boss_runtime: Option<u16>,
    model: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct CharacterAssets {
    #[serde(default)]
    costumes: Vec<CostumeRef>,
}

#[derive(Debug, Deserialize)]
struct CostumeRef {
    #[serde(rename = "ref")]
    reference: String,
}

#[derive(Debug, Deserialize)]
struct CostumeDataFile {
    id: String,
    label: String,
    model_id: Option<u16>,
    #[serde(default)]
    assets: Vec<CharacterAsset>,
    #[serde(default)]
    body_parts: Vec<CharacterBodyPart>,
}

#[derive(Debug, Deserialize)]
struct MovesetsDataFile {
    base: MovesetRef,
}

#[derive(Debug, Deserialize)]
struct MovesetRef {
    entry: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CharacterDataError {
    InvalidJson(String),
    Empty,
    MissingRequiredField { index: usize, field: &'static str },
    InvalidDirectory(String),
}

static CHARACTERS: OnceLock<Vec<Character>> = OnceLock::new();
static CHARACTER_INDEXES: OnceLock<CharacterIndexes> = OnceLock::new();

#[derive(Debug)]
struct CharacterIndexes {
    by_name: HashMap<String, usize>,
    by_id: HashMap<u16, usize>,
}

pub fn all() -> &'static [Character] {
    CHARACTERS
        .get_or_init(|| {
            parse_characters_json(EMBEDDED_CHARACTERS_JSON).expect("embedded characters json")
        })
        .as_slice()
}

pub fn initialize_data_root(root: &Path) -> Result<(), CharacterDataError> {
    let characters = read_data_root(root)?;
    let _ = CHARACTERS.set(characters);
    Ok(())
}

pub fn mark_data_unavailable() {
    let _ = CHARACTERS.set(Vec::new());
}

pub fn find(query: &str) -> Option<&'static Character> {
    let query = normalize(query);
    if query.is_empty() {
        return None;
    }

    character_indexes()
        .by_name
        .get(&query)
        .and_then(|index| all().get(*index))
}

pub fn find_by_id(id: u16) -> Option<&'static Character> {
    character_indexes()
        .by_id
        .get(&id)
        .and_then(|index| all().get(*index))
}

fn character_indexes() -> &'static CharacterIndexes {
    CHARACTER_INDEXES.get_or_init(|| {
        let mut by_name = HashMap::new();
        let mut by_id = HashMap::new();
        for (index, character) in all().iter().enumerate() {
            insert_name(&mut by_name, &character.canonical, index);
            insert_name(&mut by_name, &character.display_name, index);
            insert_name(&mut by_name, &character.model_stem, index);
            for alias in &character.aliases {
                insert_name(&mut by_name, alias, index);
            }
            for id in [
                character.model_id,
                character.playable_id,
                character.runtime_id,
                character.boss_runtime_id,
            ]
            .into_iter()
            .flatten()
            {
                by_id.entry(id).or_insert(index);
            }
        }
        CharacterIndexes { by_name, by_id }
    })
}

fn insert_name(index: &mut HashMap<String, usize>, value: &str, character_index: usize) {
    let value = normalize(value);
    if !value.is_empty() {
        index.entry(value).or_insert(character_index);
    }
}

pub fn parse_characters_json(text: &str) -> Result<Vec<Character>, CharacterDataError> {
    let characters: Vec<Character> = serde_json::from_str(text)
        .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
    validate_characters(&characters)?;
    Ok(characters)
}

pub fn read_data_root(root: &Path) -> Result<Vec<Character>, CharacterDataError> {
    let index_path = root.join("generated").join("index.json");
    if index_path.is_file() {
        return read_indexed_data_root(root, &index_path);
    }

    let characters_root = root.join("characters");
    let entries = fs::read_dir(&characters_root)
        .map_err(|error| CharacterDataError::InvalidDirectory(error.to_string()))?;
    let mut characters = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let data_path = path.join("data.json");
        if !data_path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&data_path)
            .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
        let mut character = parse_character_data_json(&path, &text)?;
        let movesets_path = path.join("movesets.json");
        if movesets_path.is_file() {
            let text = fs::read_to_string(&movesets_path)
                .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
            character.moveset_linkdata_entry = parse_moveset_entry_json(&text)?;
        }
        characters.push(character);
    }
    characters.sort_by(|left, right| left.canonical.cmp(&right.canonical));
    validate_characters(&characters)?;
    Ok(characters)
}

fn read_indexed_data_root(
    root: &Path,
    index_path: &Path,
) -> Result<Vec<Character>, CharacterDataError> {
    let text = fs::read_to_string(index_path)
        .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
    let index: CharacterIndexFile = serde_json::from_str(&text)
        .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
    let mut characters = Vec::with_capacity(index.characters.len());
    for entry in index.characters {
        let data_path = root.join(&entry.path);
        let character_dir = data_path
            .parent()
            .ok_or_else(|| CharacterDataError::InvalidDirectory(entry.path.clone()))?;
        let text = fs::read_to_string(&data_path)
            .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
        let mut character = parse_character_data_json(character_dir, &text)?;
        if let Some(movesets) = entry.movesets {
            let text = fs::read_to_string(root.join(movesets))
                .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
            character.moveset_linkdata_entry = parse_moveset_entry_json(&text)?;
        }
        characters.push(character);
    }
    characters.sort_by(|left, right| left.canonical.cmp(&right.canonical));
    validate_characters(&characters)?;
    Ok(characters)
}

fn parse_character_data_json(
    character_dir: &Path,
    text: &str,
) -> Result<Character, CharacterDataError> {
    let data: CharacterDataFile = serde_json::from_str(text)
        .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
    let primary_model = primary_model_from_costumes(character_dir, &data.assets.costumes)?;
    let costumes = character_costumes(character_dir, &data.assets.costumes)?;
    let (model_id, model_stem) = primary_model
        .map(|(id, stem)| (Some(id), stem))
        .unwrap_or((data.ids.model, fallback_model_stem(data.ids.model)));

    Ok(Character {
        playable_id: data.ids.playable,
        runtime_id: data.ids.runtime,
        boss_runtime_id: data.ids.boss_runtime,
        moveset_linkdata_entry: None,
        model_id,
        canonical: data.id,
        display_name: data.display_name,
        model_stem,
        aliases: data.aliases,
        costumes,
    })
}

fn character_costumes(
    character_dir: &Path,
    costumes: &[CostumeRef],
) -> Result<Vec<CharacterCostume>, CharacterDataError> {
    costumes
        .iter()
        .map(|costume| {
            let costume = costume_data_file(character_dir, costume)?;
            Ok(CharacterCostume {
                id: costume.id,
                label: costume.label,
                model_id: costume.model_id,
                assets: costume.assets,
                body_parts: costume.body_parts,
            })
        })
        .collect()
}

fn primary_model_from_costumes(
    character_dir: &Path,
    costumes: &[CostumeRef],
) -> Result<Option<(u16, String)>, CharacterDataError> {
    for costume in costumes {
        let costume = costume_data_file(character_dir, costume)?;
        if let Some(model) = costume.model_id.zip(primary_model_stem(&costume.assets)) {
            return Ok(Some(model));
        }
    }
    Ok(None)
}

fn costume_data_file(
    character_dir: &Path,
    costume: &CostumeRef,
) -> Result<CostumeDataFile, CharacterDataError> {
    let path = character_dir.join(&costume.reference);
    let text = fs::read_to_string(&path).map_err(|error| {
        CharacterDataError::InvalidDirectory(format!("{}: {error}", path.display()))
    })?;
    serde_json::from_str(&text).map_err(|error| CharacterDataError::InvalidJson(error.to_string()))
}

fn parse_moveset_entry_json(text: &str) -> Result<Option<u16>, CharacterDataError> {
    let data: MovesetsDataFile = serde_json::from_str(text)
        .map_err(|error| CharacterDataError::InvalidJson(error.to_string()))?;
    Ok(Some(data.base.entry))
}

fn primary_model_stem(assets: &[CharacterAsset]) -> Option<String> {
    assets
        .iter()
        .find(|asset| asset.kind == "model")
        .and_then(|asset| asset.path.as_deref())
        .map(|path| path.trim_end_matches(".g1m").to_string())
}

fn fallback_model_stem(model_id: Option<u16>) -> String {
    model_id
        .map(|id| format!("MPLC{id:03}"))
        .unwrap_or_else(|| "unknown".to_string())
}

fn validate_characters(characters: &[Character]) -> Result<(), CharacterDataError> {
    if characters.is_empty() {
        return Err(CharacterDataError::Empty);
    }

    for (index, character) in characters.iter().enumerate() {
        if character.canonical.trim().is_empty() {
            return Err(CharacterDataError::MissingRequiredField {
                index,
                field: "canonical",
            });
        }
        if character.display_name.trim().is_empty() {
            return Err(CharacterDataError::MissingRequiredField {
                index,
                field: "display_name",
            });
        }
        if character.model_stem.trim().is_empty() {
            return Err(CharacterDataError::MissingRequiredField {
                index,
                field: "model_stem",
            });
        }
    }

    Ok(())
}

fn normalize(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut last_was_sep = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            output.push('_');
            last_was_sep = true;
        }
    }
    output.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_json_loads() {
        assert!(all().len() >= 100);
    }

    #[test]
    fn exposes_linkdata_ids() {
        let law = find("law").unwrap();
        assert_eq!(law.playable_id, Some(22));
        assert_eq!(law.runtime_id, Some(26));
        assert_eq!(law.boss_runtime_id, Some(26));
        assert_eq!(law.model_id, Some(26));

        let zoro = find("zoro").unwrap();
        assert_eq!(zoro.playable_id, Some(1));
        assert_eq!(zoro.runtime_id, Some(1));
        assert_eq!(zoro.model_id, Some(1));
    }

    #[test]
    fn finds_by_aliases_and_model_stems() {
        assert_eq!(
            find("barbe blanche").map(|character| character.model_id),
            Some(Some(12))
        );
        assert_eq!(
            find("MPLC026_Law").map(|character| character.model_id),
            Some(Some(26))
        );
        assert_eq!(
            find("gear 5").map(|character| character.model_id),
            Some(Some(295))
        );
    }

    #[test]
    fn finds_by_any_known_id() {
        assert_eq!(
            find_by_id(26).map(|character| character.canonical.as_str()),
            Some("law")
        );
        assert_eq!(
            find_by_id(22).map(|character| character.canonical.as_str()),
            Some("law")
        );
        assert_eq!(
            find_by_id(107).map(|character| character.canonical.as_str()),
            Some("kaku")
        );
        assert!(find_by_id(u16::MAX).is_none());
    }

    #[test]
    fn includes_late_dlc_and_missing_mplc_entries() {
        assert_eq!(
            find("kuma").map(|character| character.model_id),
            Some(Some(17))
        );
        assert_eq!(
            find("bartolomeo").map(|character| character.model_id),
            Some(Some(37))
        );
        assert_eq!(
            find("bonney").map(|character| character.model_id),
            Some(Some(314))
        );
        assert_eq!(
            find("z").map(|character| character.model_id),
            Some(Some(327))
        );
        assert_eq!(
            find("king").map(|character| character.model_id),
            Some(Some(328))
        );
        assert_eq!(
            find("eneru").map(|character| character.model_id),
            Some(Some(322))
        );
    }

    #[test]
    fn includes_linkdata_npc_rows_with_model_ids() {
        let kaku = find("kaku").unwrap();
        assert_eq!(kaku.model_id, Some(58));
        assert_eq!(kaku.runtime_id, Some(107));
        assert_eq!(kaku.boss_runtime_id, Some(80));
        assert_eq!(
            find("jabra").map(|character| character.model_id),
            Some(Some(59))
        );
        assert_eq!(
            find("blueno").map(|character| character.model_id),
            Some(Some(60))
        );
        assert_eq!(
            find("bon clay").map(|character| character.model_id),
            Some(Some(56))
        );
    }

    #[test]
    fn includes_known_moveset_linkdata_entries() {
        assert_eq!(
            find("zoro").and_then(|character| character.moveset_linkdata_entry),
            Some(69)
        );
        assert_eq!(
            find("luffy_bounceman").and_then(|character| character.moveset_linkdata_entry),
            Some(208)
        );
        assert_eq!(
            find("luffy_snakeman").and_then(|character| character.moveset_linkdata_entry),
            Some(209)
        );
        assert_eq!(
            find("linlin").and_then(|character| character.moveset_linkdata_entry),
            Some(106)
        );
        assert_eq!(
            find("big_mom_temperamented").and_then(|character| character.moveset_linkdata_entry),
            Some(212)
        );
        assert_eq!(
            find("kaido").and_then(|character| character.moveset_linkdata_entry),
            Some(108)
        );
        assert_eq!(
            find("kaido_d2").and_then(|character| character.moveset_linkdata_entry),
            Some(213)
        );
        assert_eq!(
            find("oden").and_then(|character| character.moveset_linkdata_entry),
            Some(233)
        );
        assert_eq!(
            find("urouge").and_then(|character| character.moveset_linkdata_entry),
            Some(229)
        );
        assert_eq!(
            find("kiku").and_then(|character| character.moveset_linkdata_entry),
            Some(231)
        );
        assert_eq!(
            find("bounceman").and_then(|character| character.model_id),
            None
        );
        assert_eq!(
            find("garp").and_then(|character| character.moveset_linkdata_entry),
            None
        );
        assert_eq!(
            find("garp_yng").and_then(|character| character.moveset_linkdata_entry),
            Some(247)
        );
        assert_eq!(
            find("rayleigh_yng").and_then(|character| character.moveset_linkdata_entry),
            Some(248)
        );
    }

    #[test]
    fn parser_rejects_empty_data() {
        assert_eq!(parse_characters_json("[]"), Err(CharacterDataError::Empty));
    }

    #[test]
    fn reads_split_oppw4_data_root() {
        let root = temp_data_root("split-data");
        let character_dir = root.join("characters").join("law");
        fs::create_dir_all(character_dir.join("costumes")).expect("costume dir");
        fs::write(
            character_dir.join("data.json"),
            r#"
                {
                  "id": "law",
                  "display_name": "Law",
                  "aliases": ["trafalgar_law"],
                  "ids": {
                    "playable": 22,
                    "runtime": 26,
                    "boss_runtime": 26,
                    "model": 26
                  },
                  "assets": {
                    "costumes": [
                      { "id": "default", "ref": "costumes/default.json" }
                    ]
                  }
                }
            "#,
        )
        .expect("character data");
        fs::write(
            character_dir.join("costumes").join("default.json"),
            r#"
                {
                  "character_id": "law",
                  "id": "default",
                  "label": "Default",
                  "slot": 1,
                  "model_id": 26,
                  "assets": [
                    {
                      "kind": "model",
                      "label": "Default character model",
                      "path": "MPLC026_Law.g1m"
                    }
                  ]
                }
            "#,
        )
        .expect("costume data");
        fs::write(
            character_dir.join("movesets.json"),
            r#"
                {
                  "character_id": "law",
                  "base": {
                    "linkdata_file": "LINKDATA_A",
                    "entry": 90
                  },
                  "variants": []
                }
            "#,
        )
        .expect("moveset data");

        let characters = read_data_root(&root).expect("oppw4-data root");
        let law = characters
            .iter()
            .find(|character| character.canonical == "law")
            .expect("law");

        assert_eq!(characters.len(), 1);
        assert_eq!(law.playable_id, Some(22));
        assert_eq!(law.model_id, Some(26));
        assert_eq!(law.model_stem, "MPLC026_Law");
        assert_eq!(law.moveset_linkdata_entry, Some(90));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn indexed_data_root_preserves_costume_assets_body_parts_and_missing_movesets() {
        let root = temp_data_root("indexed-data");
        let character_dir = root.join("characters").join("garp");
        fs::create_dir_all(root.join("generated")).expect("generated dir");
        fs::create_dir_all(character_dir.join("costumes")).expect("costume dir");
        fs::write(
            root.join("generated").join("index.json"),
            r#"
                {
                  "characters": [
                    {
                      "path": "characters/garp/data.json"
                    }
                  ]
                }
            "#,
        )
        .expect("index");
        fs::write(
            character_dir.join("data.json"),
            r#"
                {
                  "id": "garp",
                  "display_name": "Garp",
                  "aliases": ["hero_of_the_marines"],
                  "ids": {
                    "playable": null,
                    "runtime": 310,
                    "boss_runtime": 311,
                    "model": 9
                  },
                  "assets": {
                    "costumes": [
                      { "id": "young", "ref": "costumes/young.json" }
                    ]
                  }
                }
            "#,
        )
        .expect("character data");
        fs::write(
            character_dir.join("costumes").join("young.json"),
            r#"
                {
                  "character_id": "garp",
                  "id": "young",
                  "label": "Young",
                  "slot": 2,
                  "model_id": 9,
                  "assets": [
                    {
                      "kind": "model",
                      "label": "Young model",
                      "path": "MPLC009_GarpYoung.g1m"
                    },
                    {
                      "kind": "portrait",
                      "label": "Young portrait",
                      "path": "ui/garp_young.dds"
                    }
                  ],
                  "body_parts": [
                    {
                      "id": "body",
                      "label": "Body",
                      "assets": [
                        {
                          "kind": "texture",
                          "path": "MPLC009_GarpYoung_Body.g1t"
                        }
                      ]
                    },
                    {
                      "id": "weapon_02",
                      "label": "Second weapon",
                      "assets": [
                        {
                          "kind": "texture",
                          "path": "MPLC009_GarpYoung_Weapon02.g1t"
                        }
                      ]
                    }
                  ]
                }
            "#,
        )
        .expect("costume data");

        let characters = read_data_root(&root).expect("indexed oppw4-data root");
        let garp = characters
            .iter()
            .find(|character| character.canonical == "garp")
            .expect("garp");

        assert_eq!(characters.len(), 1);
        assert_eq!(garp.moveset_linkdata_entry, None);
        assert_eq!(garp.model_stem, "MPLC009_GarpYoung");
        assert_eq!(garp.costumes.len(), 1);

        let costume = &garp.costumes[0];
        assert_eq!(costume.id, "young");
        assert_eq!(costume.assets.len(), 2);
        assert_eq!(costume.body_parts.len(), 2);
        assert_eq!(costume.body_parts[0].id, "body");
        assert_eq!(
            costume.body_parts[0].assets[0].path.as_deref(),
            Some("MPLC009_GarpYoung_Body.g1t")
        );
        assert_eq!(costume.body_parts[1].id, "weapon_02");
        assert_eq!(
            costume.body_parts[1].assets[0].path.as_deref(),
            Some("MPLC009_GarpYoung_Weapon02.g1t")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_workspace_oppw4_data_submodule() {
        let root = workspace_data_root();
        let characters = read_data_root(&root).expect("workspace oppw4-data root");
        let law = characters
            .iter()
            .find(|character| character.canonical == "law")
            .expect("law");
        let garp = characters
            .iter()
            .find(|character| character.canonical == "garp")
            .expect("garp");
        let garp_yng = characters
            .iter()
            .find(|character| character.canonical == "garp_yng")
            .expect("garp_yng");

        assert!(characters.len() >= 100);
        assert_eq!(law.model_stem, "MPLC026_Law");
        assert_eq!(law.moveset_linkdata_entry, Some(90));
        assert_eq!(garp.moveset_linkdata_entry, None);
        assert_eq!(garp_yng.moveset_linkdata_entry, Some(247));
    }

    #[test]
    fn rejects_unknown_names() {
        assert!(find("").is_none());
        assert!(find("not a real character").is_none());
    }

    fn temp_data_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-data-{label}-{nanos}"))
    }

    fn workspace_data_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("..")
            .join("oppw4-data")
    }
}
