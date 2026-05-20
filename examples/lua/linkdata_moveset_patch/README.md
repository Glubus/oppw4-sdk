# LinkData Moveset Patch Example

Lua mod showing how `moveset_patcher` can register a LinkData moveset payload
without editing `LINKDATA_A.BIN` on disk.

This example targets `garp_yng`; the base `garp` character intentionally does
not own the 247 moveset entry in the SDK data.

Validate the manifest:

```sh
cargo run -p mod-manifest-tool -- examples/lua/linkdata_moveset_patch/mod.toml
```

Install this under `moveset_patcher/mods/` so the declared plugin dependency and
Lua module are available.
