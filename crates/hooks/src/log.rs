use std::fmt::Display;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

static LOGGER: OnceLock<fn(String)> = OnceLock::new();
static DIAGNOSTICS_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_logger(logger: fn(String)) {
    let _ = LOGGER.set(logger);
}

pub fn set_diagnostics_enabled(enabled: bool) {
    DIAGNOSTICS_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn diagnostics_enabled() -> bool {
    DIAGNOSTICS_ENABLED.load(Ordering::Relaxed)
}

pub fn write_line(message: impl Display) {
    if let Some(logger) = LOGGER.get() {
        logger(message.to_string());
    }
}
