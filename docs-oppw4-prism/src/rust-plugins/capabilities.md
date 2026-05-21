# Capabilities

Capabilities are the guardrail between declared plugin intent and runtime behavior.

Common capabilities:

- `lua.module`: register Lua modules.
- `std.character.extend`: extend `std.character` with plugin methods.
- `files.virtualize`: register generic virtual file providers.
- `rdb.patch`: register RDB virtual or patch providers.
- `linkdata.patch`: patch LinkData entries or rows.
- `hooks.install`: install hooks.
- `memory.read`: read process memory.
- `memory.scan`: scan process memory.
- `memory.write`: write process memory.
- `signals.subscribe`: subscribe to SDK signals.
- `signals.emit`: emit SDK signals.
- `config.schema`: register plugin config schemas/defaults.

The loader does not understand these high-level names. SDK core owns capability validation.
