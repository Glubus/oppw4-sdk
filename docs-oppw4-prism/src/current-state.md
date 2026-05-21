# Current State

The current SDK architecture is usable as a split loader plus SDK prototype.

Implemented:

- loader consumes `plugins/sdk/sdk.dll`;
- SDK service DLLs live under `plugins/sdk/`;
- official plugins live under `plugins/<plugin_id>/`;
- Lua mods run through the SDK sandbox;
- `std.*` modules are available to Lua mods;
- `oppw4-data` is a mandatory data tree and can be improved without recompiling;
- plugin manifests declare dependencies, Lua modules, and capabilities;
- ABI tables use `struct_size`;
- config schema/default registration exists for plugins.

Still evolving:

- character data completeness;
- game-specific reverse-engineering for features like difficulty extension;
- public examples and polished modder workflows.

Target platform is Windows. Local Linux-native tests are useful for pure crates,
but official game-facing plugins are validated against `x86_64-pc-windows-gnu`.
