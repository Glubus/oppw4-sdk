#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeError {
    EmptyIdentifier {
        field: &'static str,
    },
    LoadFailed {
        mod_id: String,
        errors: Vec<BridgeError>,
    },
    MismatchedModId {
        expected: String,
        actual: String,
    },
    MismatchedBridgeId {
        expected: String,
        actual: String,
    },
    BridgeLoadError {
        mod_id: String,
        bridge_id: String,
        message: String,
    },
    MissingBridge {
        bridge_id: String,
    },
    NoBridgeForMod {
        mod_id: String,
        entry_file: String,
    },
    AmbiguousBridgeForMod {
        mod_id: String,
        entry_file: String,
    },
}
