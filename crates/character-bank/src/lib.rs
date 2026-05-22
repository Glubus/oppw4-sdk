pub mod characters;
pub mod game;
pub mod missions;

pub use characters::{
    all, find, find_by_id, parse_characters_json, read_data_root, Character, CharacterAsset,
    CharacterBodyPart, CharacterCostume, CharacterDataError,
};

pub fn initialize_data_root(root: &std::path::Path) -> Result<(), CharacterDataError> {
    characters::initialize_data_root(root)?;
    let _ = missions::initialize_data_root(root);
    Ok(())
}

pub fn mark_data_unavailable() {
    characters::mark_data_unavailable();
    missions::mark_data_unavailable();
}
