# Lua Standard Library

The SDK standard library lives under `std.*`.

Available modules:

- `std.character`
- `std.files`
- `std.path`
- `std.math`
- `std.time`
- `std.collections`
- `std.buffer`
- `std.log`
- `std.mod`

Import modules with:

```lua
local character = require("std.character")
```

Most modules are also available as `std.<name>` after runtime initialization.
