# Asset Replacement Example

Lua mod showing the intended folder shape for model and portrait replacement
assets.

```text
asset_replacement/
  mod.toml
  mod.lua
  assets/
    costumes/
      default/
        MPLC001_Zoro_Custom.g1m
    portraits/
      zoro_custom.dds
```

Validate the manifest:

```sh
cargo run -p mod-manifest-tool -- examples/lua/asset_replacement/mod.toml
```

Install this under the game-level `mods/` directory and add the referenced
assets before trying it in-game.
