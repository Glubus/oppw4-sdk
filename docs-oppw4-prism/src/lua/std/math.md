# std.math

`std.math` provides deterministic helpers that are useful in mods.

```lua
local mathx = require("std.math")

mathx.clamp(value, min, max)
mathx.saturate(value)
mathx.lerp(start, finish, t)
mathx.inverse_lerp(start, finish, value)
mathx.remap(in_min, in_max, out_min, out_max, value)
mathx.round_to(value, step)
mathx.align_down(value, alignment)
mathx.align_up(value, alignment)
```

Lua's built-in `math` library remains available separately.
