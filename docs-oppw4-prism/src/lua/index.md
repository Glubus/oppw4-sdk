# Lua Modding Overview

Lua mods are runtime mods loaded from the game-level `mods/` directory. They are
not SDK plugins.

A Lua mod usually contains:

```text
mods/my_mod/
  mod.toml
  mod.lua
  assets/
```

Lua runs in the SDK sandbox. Safe Lua libraries such as `string`, `table`,
`math`, and `utf8` remain available. Dangerous globals such as `os`, `io`,
`debug`, and global `package` are hidden.

Use `require("std.<module>")` for SDK APIs.
