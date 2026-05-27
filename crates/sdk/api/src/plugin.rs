use crate::{plugin_abi_from_raw, LogPolicy, Oppw4PluginApi, PluginContext, PluginResult};

pub trait Plugin {
    const ID: &'static str;

    fn log_policy() -> LogPolicy {
        LogPolicy::HOST
    }

    fn init(context: PluginContext<'_>) -> PluginResult<()>;
}

/// Experimental trait for native Rust mods.
///
/// Native Rust mods use the same registry-first host API as plugins, but they
/// are intended for advanced authors because they ship native DLL code.
pub trait RustMod {
    const ID: &'static str;
    const NAME: &'static str;

    fn log_policy() -> LogPolicy {
        LogPolicy::HOST
    }

    fn register(context: PluginContext<'_>) -> PluginResult<()>;
}

pub struct RustModPlugin<M>(std::marker::PhantomData<M>);

impl<M: RustMod> Plugin for RustModPlugin<M> {
    const ID: &'static str = M::ID;

    fn log_policy() -> LogPolicy {
        M::log_policy()
    }

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context.log(format!("native rust mod loaded: {}", M::NAME));
        M::register(context)
    }
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

    struct TestRustMod;

    impl RustMod for TestRustMod {
        const ID: &'static str = "test_rust_mod";
        const NAME: &'static str = "Test Rust Mod";

        fn register(_context: PluginContext<'_>) -> PluginResult<()> {
            Ok(())
        }
    }

    #[test]
    fn rust_mod_plugin_uses_mod_identity() {
        assert_eq!(RustModPlugin::<TestRustMod>::ID, "test_rust_mod");
    }
}
