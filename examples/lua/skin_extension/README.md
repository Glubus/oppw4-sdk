# Skin Extension Example

Lua mod showing how a plugin can extend `std.character` handles. The
`skin_patcher` plugin registers `replace_costume` and `replace_portrait` when it
is required by a mod.

Validate the manifest:

```sh
cargo run -p mod-manifest-tool -- examples/lua/skin_extension/mod.toml
```

Install this under `skin_patcher/mods/` so the declared plugin dependency and
Lua module are available.
