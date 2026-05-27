use std::sync::OnceLock;

static LOGGER: OnceLock<fn(String)> = OnceLock::new();

pub fn set_logger(logger: fn(String)) {
    let _ = LOGGER.set(logger);
}

pub(crate) fn write_line(message: impl AsRef<str>) {
    if let Some(logger) = LOGGER.get() {
        logger(message.as_ref().to_string());
    }
}
