# Player Event JS Example

This is the recommended mod shape: `mod.toml` declares an entry file, the JS
bridge loads `main.js`, and callbacks are registered through `oppw4.on`.

The example intentionally talks only to the bridge/registry surface. It does not
know how the runtime plugin implemented the event.

`oppw4.registry.modules()` returns the registry-selected modules for this mod.
The JS code can inspect what the host gave it without hardcoding Rust plugin
details.
