# Plugin Development

Plugins are native DLLs loaded by the SDK host. They use `plugin-sdk` and must
declare their capabilities in `plugin.toml`.

## Minimal Plugin

```rust
use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct MyPlugin;

impl Plugin for MyPlugin {
    const ID: &'static str = "my_plugin";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context.log("my_plugin initialized");
        Ok(())
    }
}

export_plugin!(MyPlugin);
```

## Registry Modules

Plugins expose mod-facing APIs by registering registry modules. The host stores
module metadata and language bridges decide how to install those modules into
their VM.

Use `RegistryModuleFeature` or `registry_module!` for normal registration.
Plugins must not depend on a concrete bridge such as JS or Python.

## Experimental Native Rust Mods

Native Rust mods use `RustMod` and `export_rust_mod!`. This is an advanced path:
authors ship native DLL code and must understand ABI/toolchain constraints.
JavaScript mods remain the recommended path.
