Put LinkData entry payload patches here.

Example:

```text
0247.bin
0247.hex
entry_0247.bin
```

The file content must be the raw inflated entry payload, not a full LINKDATA_A.BIN.

Lua mods can also register movesets without editing the game file on disk:

```lua
local moveset_patcher = require("moveset_patcher")
local character = require("std.character")

local garp_moveset = moveset_patcher.moveset({
  payload_file = "0247.hex",
})

character.find("garp"):replace_movesets(garp_moveset)
```

For readable experiments, `moveset()` also accepts a structured payload. The
fields are still raw u32 words until the format is fully named, but this is
easier to diff/edit than a giant binary blob:

```lua
local moveset_patcher = require("moveset_patcher")
local character = require("std.character")

local garp_moveset = moveset_patcher.moveset({
  section_count = 18,
  sections = {
    {
      index = 0,
      record_size = 16,
      records = {
        { 0x0000a410, 0, 0, 0x0000a410 },
      },
    },
  },
})

character.find("garp"):replace_movesets(garp_moveset)
```
