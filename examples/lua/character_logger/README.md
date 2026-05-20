# Character Logger

Minimal Lua mod that loads SDK standard modules, reads character metadata, and
writes mod-scoped log entries.

Validate the manifest:

```sh
cargo run -p mod-manifest-tool -- examples/lua/character_logger/mod.toml
```

To try it in-game, copy this folder into a plugin `mods/` directory.
