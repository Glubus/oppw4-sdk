# SDK Examples

Examples are split by what a modder is building.

## JavaScript Mods

Use `examples/js/` for the recommended mod path. JavaScript mods run through
`bridges/js`, receive registry events, and call registry modules exposed by
official plugins.

- `player_event`: registers an event callback and reads available registry
  context from the callback payload.
- `character_registry_probe`: imports `character` from the `sdk` registry
  projection and logs known character rows.

Validate JS mod manifests with:

```sh
cargo run -p mod-manifest-tool -- examples/js/player_event/mod.toml
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

## Experimental Rust Native Mods

Use `examples/rust/native_mod` only when a mod needs native Rust and the author
accepts the ABI/toolchain risk. This path uses the same registry-first SDK
surface as plugins, but is intentionally documented as advanced.

Validate source manifests before the DLL exists with:

```sh
cargo run -p mod-manifest-tool -- --manifest-only examples/rust/native_mod/mod.toml
```
