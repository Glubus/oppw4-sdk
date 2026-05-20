use crate::{plugin_abi_from_raw, LogPolicy, Oppw4PluginApi, PluginContext, PluginResult};

pub trait Plugin {
    const ID: &'static str;

    fn log_policy() -> LogPolicy {
        LogPolicy::HOST
    }

    fn init(context: PluginContext<'_>) -> PluginResult<()>;
}

/// # Safety
///
/// `api` must either be null or point to a valid host-owned [`Oppw4PluginApi`]
/// table for the duration of plugin initialization.
pub unsafe fn init_plugin<P: Plugin>(api: *const Oppw4PluginApi) -> i32 {
    let api = match unsafe { plugin_abi_from_raw(api) } {
        Ok(api) => api,
        Err(error) => return error.code(),
    };
    let context = match PluginContext::new::<P>(api) {
        Ok(context) => context,
        Err(_) => return -2,
    };
    match P::init(context) {
        Ok(()) => 0,
        Err(error) => {
            context.log(format!("plugin init failed: {error}"));
            -3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginResult;

    struct TestPlugin;

    impl Plugin for TestPlugin {
        const ID: &'static str = "test_plugin";

        fn init(_context: PluginContext<'_>) -> PluginResult<()> {
            Ok(())
        }
    }

    #[test]
    fn init_plugin_rejects_null_api() {
        assert_eq!(unsafe { init_plugin::<TestPlugin>(std::ptr::null()) }, -1);
    }
}
