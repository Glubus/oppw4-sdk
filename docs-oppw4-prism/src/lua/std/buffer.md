# std.buffer

`std.buffer` builds and reads binary payloads.

```lua
local buffer = require("std.buffer")

local writer = buffer.writer()
writer:u8(0x12)
writer:u16_le(0x3456)
writer:u32_le(0x789abcde)
writer:i32_le(-2)
writer:bytes({ 1, 2, 3 })
writer:align(16, 0)

local payload = writer:to_string()
local bytes = writer:to_bytes()
```

Reader:

```lua
local reader = buffer.reader(payload)
reader:u8()
reader:u16_le()
reader:u32_le()
reader:i32_le()
reader:bytes(4)
reader:remaining()
reader:position()
reader:seek(1)
```

Writers validate numeric ranges before writing.
