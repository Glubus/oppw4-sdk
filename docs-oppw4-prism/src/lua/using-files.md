# Using Files

Use `std.files` for files inside the current mod.

```lua
local files = require("std.files")

local text = files.read_text("config.lua")
local bytes = files.read_bytes("payload.bin")
```

Use `std.path` for path string manipulation.

```lua
local path = require("std.path")

local file = path.join("assets", "garp", "body.g1t")
```

Do not use `io` or absolute filesystem paths in mod scripts.
