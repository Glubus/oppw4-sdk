use plugin_sdk::OwnedHostApi;

pub(crate) trait RuntimeExposure {
    type Config;

    fn install(host: OwnedHostApi, config: Self::Config);
}
