# Config Schema

Plugins can register config schemas so tools and users know what config files are expected.

Use `host.configs()`:

```rust
host.configs().register_schema(
    "my_plugin",
    "config",
    r#"{"type":"object","additionalProperties":false}"#,
)?;
```

Requirements:

- the plugin must request `config.schema`;
- schema name must be present;
- schema body must be present;
- duplicate schemas for the same plugin/name are rejected.

This does not force a plugin to use JSON forever. It gives the SDK a stable place to expose defaults and validation metadata.
