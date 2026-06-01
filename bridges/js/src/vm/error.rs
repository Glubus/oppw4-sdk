use std::fmt::{Debug, Display};

pub(super) fn js(from: &'static str, to: &'static str, message: impl Display) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message(from, to, message.to_string())
}

pub(super) fn js_debug(
    from: &'static str,
    to: &'static str,
    prefix: &'static str,
    error: impl Debug,
) -> rquickjs::Error {
    js(from, to, format!("{prefix}: {error:?}"))
}

pub(super) trait StringContext<T> {
    fn context(self, message: &str) -> Result<T, String>;
}

impl<T, E> StringContext<T> for Result<T, E>
where
    E: Display,
{
    fn context(self, message: &str) -> Result<T, String> {
        self.map_err(|error| format!("{message}: {error}"))
    }
}
