# plugin.toml

`plugin.toml` declares static intent before the DLL runs.

```toml
[plugin]
id = "my_plugin"
version = "0.1.0"
entry = "my_plugin.dll"

[dependencies]
plugins = []

[lua]
modules = ["my_plugin"]

[capabilities]
requires = ["lua.module"]
provides = []
```

Important rules:

- `plugin.id` is the stable runtime identity;
- `entry` is the DLL loaded by SDK core;
- `[dependencies].plugins` controls plugin load order;
- `[lua].modules` must list each Lua module registered at runtime;
- capability names are normalized and validated;
- runtime operations can fail when the manifest did not request the required capability.

Configs belong under `plugins/configs/<plugin_id>/`, not inside `mods/`.
