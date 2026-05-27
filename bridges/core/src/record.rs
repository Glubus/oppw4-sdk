use crate::{BridgeId, BridgeLoadReport, ModId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModLifecycle {
    BootOnce,
    EventDriven,
}

impl ModLifecycle {
    pub(crate) fn infer(report: &BridgeLoadReport) -> Self {
        if report.handlers.is_empty() {
            Self::BootOnce
        } else {
            Self::EventDriven
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModRecord {
    pub mod_id: ModId,
    pub bridge_id: BridgeId,
    pub lifecycle: ModLifecycle,
}
