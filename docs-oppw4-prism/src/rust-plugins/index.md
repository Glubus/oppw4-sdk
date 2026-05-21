# Rust Plugins

Rust plugins are DLLs loaded by SDK core.

They should use the `plugin-sdk` crate instead of calling raw ABI functions directly. The SDK wrapper handles API version checks, `struct_size`, host service access, and readable errors.

Minimal shape:

```rust
use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        host.log().info("my_plugin loaded")?;
        Ok(())
    }
}

export_plugin!(MyPlugin);
```

Build target for runtime use is Windows GNU:

```text
x86_64-pc-windows-gnu
```
