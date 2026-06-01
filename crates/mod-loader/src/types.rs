use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub runtime: Option<String>,
    pub uses_plugins: Vec<String>,
    pub entry_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredMod {
    pub manifest: ModManifest,
    pub source: ModSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModSource {
    Directory(PathBuf),
    Zip { path: PathBuf, root: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModManifestError {
    InvalidToml,
    MissingModTable,
    MissingId,
    MissingEntry,
    InvalidEntryPath,
}
