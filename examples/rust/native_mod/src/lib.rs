use plugin_sdk::{export_rust_mod, PluginContext, PluginResult, RustMod};

struct NativeModExample;

impl RustMod for NativeModExample {
    const ID: &'static str = "native_mod_example";
    const NAME: &'static str = "Native Rust Mod Example";

    fn register(context: PluginContext<'_>) -> PluginResult<()> {
        context.log("native rust mod initialized");
        Ok(())
    }
}

export_rust_mod!(NativeModExample);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_experimental_mod_identity() {
        assert_eq!(NativeModExample::ID, "native_mod_example");
        assert_eq!(NativeModExample::NAME, "Native Rust Mod Example");
    }
}
