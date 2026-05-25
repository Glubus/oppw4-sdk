# Skin Extension Example

Lua mod showing how a plugin can extend `std.character` handles. The
`sdk.rdb.patcher` SDK module registers `replace_costume` and
`replace_portrait` when it is required by a mod.

Validate the manifest:

```sh
cargo run -p mod-manifest-tool -- examples/lua/skin_extension/mod.toml
```

Install this under the game-level `mods/` directory so the SDK can load it.
