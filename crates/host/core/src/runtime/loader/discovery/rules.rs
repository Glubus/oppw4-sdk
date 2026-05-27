use std::collections::HashSet;

pub(super) fn dependencies_loaded(dependencies: &[String], loaded: &HashSet<String>) -> bool {
    dependencies.iter().all(|dependency| {
        loaded
            .iter()
            .any(|loaded_id| loaded_id.eq_ignore_ascii_case(dependency))
    })
}

pub(super) fn capabilities_available(required: &[String], available: &HashSet<String>) -> bool {
    required
        .iter()
        .all(|required_capability| has_capability(required_capability, available))
}

pub(super) fn has_capability(required: &str, available: &HashSet<String>) -> bool {
    available
        .iter()
        .any(|capability| capability.eq_ignore_ascii_case(required))
}
