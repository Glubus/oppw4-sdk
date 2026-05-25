# SDK Examples

Examples are split by what a modder is building.

## Lua Mods

Use `examples/lua/` when the mod only needs SDK Lua modules and plugin-provided
Lua APIs.

- `character_logger`: reads `std.character`, `std.mod`, and `std.log`.
- `skin_extension`: consumes `sdk.rdb.patcher` methods added to `std.character`
  by the `sdk_rdb` service.
- `linkdata_moveset_patch`: consumes `moveset_patcher` to register a LinkData
  moveset payload.
- `asset_replacement`: consumes `sdk.rdb.patcher` for model and portrait
  replacement through the `sdk_rdb` service.

Validate Lua mod manifests with:

```sh
cargo run -p mod-manifest-tool -- examples/lua/character_logger/mod.toml
```

## Rust Plugins

Use `examples/rust/` when the mod needs a native plugin DLL and direct SDK host
APIs.

- `log_plugin`: exports `oppw4_plugin_init`, logs during startup, and reads
  optional game status.

Validate plugin manifests with:

```sh
cargo run -p plugin-manifest-tool -- examples/rust/log_plugin/plugin.toml
```

Build Rust plugins for the game with the default Windows MSVC toolchain:

```sh
cargo build -p oppw4-log-example-plugin
```
