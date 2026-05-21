# std.collections

`std.collections` provides predictable utility structures.

```lua
local collections = require("std.collections")

local map = collections.map()
map:set("garp", 12)
map:get("garp")
map:get_or("missing", 0)
map:has("garp")
map:remove("garp")
map:len()
map:entries()
```

Ring buffer:

```lua
local history = collections.ring_buffer(60)
history:push(sample)
history:last()
history:values()
```

`map` accepts string, integer, and boolean keys. `ring_buffer` returns values
from oldest to newest.
