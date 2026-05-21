# std.mod

`std.mod` exposes current mod metadata.

```lua
local mod = require("std.mod")

local current = mod.current()
print(current.id)
print(current.name)
print(current.is_zip)
```

Convenience helpers:

```lua
mod.id()
mod.name()
mod.root()
mod.is_zip()
```
