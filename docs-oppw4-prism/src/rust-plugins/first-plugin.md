# First Rust Plugin

Create a plugin directory under `plugins/<plugin_id>/` for runtime packages, or under `official_plugins/` when it is part of this repository.

Every Rust plugin needs:

- a DLL crate;
- a `plugin.toml`;
- one exported plugin entry through `export_plugin!`;
- explicit capabilities for host operations.

Example init flow:

```rust
fn init(context: PluginContext<'_>) -> PluginResult<()> {
    let host = context.host();
    host.log().info("loaded")?;
    Ok(())
}
```

Use host services from `context.host()` and keep plugin state inside the plugin. Do not reach into loader internals.
