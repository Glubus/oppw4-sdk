# Plugin Development

This SDK exposes host services through `plugin-sdk`. Third-party plugins should
use the SDK API instead of calling loader internals or inventing private runtime
folders.

## Runtime Layout

End-user packages are copied into the game folder:

```text
OPPW4/
  dinput8.dll
  oppw4-data/
  plugins/
    sdk/
      sdk.dll
      runtime.dll
      linkdata.dll
      rdb.dll
    my_plugin/
      my_plugin.dll
      plugin.toml
  mods/
```

Runtime mods belong only under the game-level `mods/` directory. Plugin config
belongs under `plugins/configs/<plugin_id>/`.

## Manifest

Plugins declare dependencies, Lua modules, and capabilities in `plugin.toml`:

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

SDK core refuses critical runtime operations when the matching capability is not
declared.

## Host Services

Use `context.host()` from `Plugin::init`:

- `host.paths()` for game, plugin, mods, and config roots;
- `host.configs()` for config schema/default registration;
- `host.log()` for plugin logs;
- `host.lua()` to register Lua modules;
- `host.files()` for generic virtual file providers;
- `host.linkdata()` for LinkData patches;
- `host.rdb()` for RDB virtual file and patch providers;
- `host.signals()` for named signal subscribe/emit;
- `host.memory()` and `host.hooks()` only with explicit capabilities.

Do not mutate Lua globals directly. Register Lua modules through `host.lua()`
and expose mod-facing behavior from those modules.

## Examples

Examples live under `examples/` and are not copied into release packages. Use
them as templates, then package the resulting plugin folder under `plugins/`.
