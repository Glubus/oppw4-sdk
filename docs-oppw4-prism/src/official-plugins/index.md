# Official Plugins

Official plugins are shipped with the SDK because they provide the game-specific services and the first modding features.

They are still normal plugins: each plugin has its own `plugin.toml`, declares capabilities, and talks to SDK core through `plugin-sdk`.

Current split:

- `sdk_core`: the SDK core plugin entry.
- `sdk_runtime`: game runtime probes, game status, active character providers, and runtime-oriented Lua helpers.
- `sdk_linkdata`: LinkData service and patch routing.
- `sdk_rdb`: RDB service and virtual/patch routing.
- `skin_patcher`: model and texture replacement helpers.
- `moveset_patcher`: LinkData moveset patch helpers.
- `fx_director`: difficulty/effect experiments and runtime probes.

The goal is to keep game format knowledge out of the loader and out of generic SDK core. When a feature needs to understand LinkData, RDB, characters, or runtime memory, it belongs in an SDK service or an official plugin.
