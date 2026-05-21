# std.files

`std.files` reads files from the current mod.

```lua
local files = require("std.files")

local text = files.read_text("moveset.lua")
local bytes = files.read_bytes("payload.bin")
```

It supports directory mods and zip mods through the SDK. Paths must be relative
to the current mod.
