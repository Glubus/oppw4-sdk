use std::sync::OnceLock;

use plugin_sdk::OwnedHostApi;

const PLUGIN_ID: &str = "sdk.linkdata";

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn initialize(host: OwnedHostApi) {
    let _ = HOST.set(host);
}

pub(crate) fn write_line(message: impl AsRef<str>) {
    if let Some(host) = HOST.get() {
        let _ = host.log().write(PLUGIN_ID, message.as_ref());
    }
}
