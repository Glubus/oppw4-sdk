# First Lua Mod

Minimal directory mod:

```text
mods/hello_prism/
  mod.toml
  mod.lua
```

`mod.toml`:

```toml
[mod]
id = "hello_prism"
name = "Hello Prism"
version = "0.1.0"
entry = "mod.lua"
```

`mod.lua`:

```lua
local log = require("std.log")
local mod = require("std.mod")

log.info("loaded " .. mod.id())
```

Mods should use SDK APIs instead of raw filesystem or process APIs.
