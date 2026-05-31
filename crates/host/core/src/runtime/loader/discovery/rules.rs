use std::collections::HashSet;

pub(super) fn dependencies_loaded(dependencies: &[String], loaded: &HashSet<String>) -> bool {
    dependencies
        .iter()
        .all(|dependency| loaded.contains(&dependency.to_ascii_lowercase()))
}

pub(super) fn capabilities_available(required: &[String], available: &HashSet<String>) -> bool {
    required
        .iter()
        .all(|required_capability| has_capability(required_capability, available))
}

pub(super) fn has_capability(required: &str, available: &HashSet<String>) -> bool {
    available.contains(&required.to_ascii_lowercase())
}
