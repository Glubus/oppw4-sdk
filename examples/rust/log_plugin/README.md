# Log Example Plugin

Minimal Rust plugin that exports `oppw4_plugin_init`, logs during startup, and
reads optional game status from the SDK host API.

Validate the manifest:

```sh
cargo run -p plugin-manifest-tool -- examples/rust/log_plugin/plugin.toml
```

Build with the default Windows MSVC toolchain:

```sh
cargo build -p oppw4-log-example-plugin
```

Copy `log_example.dll` and `plugin.toml` into a plugin folder to try it in-game.
