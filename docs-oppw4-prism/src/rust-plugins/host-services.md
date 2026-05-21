# Host Services

`HostApi` groups SDK core services behind small service objects.

Current service areas:

- `host.paths()`: game root, plugin root, mods root, config root.
- `host.configs()`: config schema/default registration.
- `host.log()`: plugin logs.
- `host.lua()`: Lua module registration.
- `host.files()`: generic virtual file providers.
- `host.linkdata()`: LinkData patch requests.
- `host.rdb()`: RDB virtual and patch providers.
- `host.signals()`: named signal subscribe/emit.
- `host.game()`: game status and active character routing.
- `host.memory()`: memory read/write/scan primitives.
- `host.hooks()`: hook installation capability checks.

Prefer the most specific service. For example, RDB replacement should go through `host.rdb()`, not a generic `host.files()` provider.
