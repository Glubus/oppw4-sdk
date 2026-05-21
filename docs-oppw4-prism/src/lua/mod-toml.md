# mod.toml

`mod.toml` declares a Lua mod.

```toml
[mod]
id = "garp_skin"
name = "Garp Skin"
version = "0.1.0"
entry = "mod.lua"
```

Rules:

- `id` should be stable and lowercase;
- `entry` points to the Lua script inside the mod;
- dependencies should be declared when the mod expects official plugin modules.

Runtime mods belong under `mods/`, not under `plugins/`.
