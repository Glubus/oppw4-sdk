# std.time

`std.time` exposes monotonic runtime timing helpers without exposing Lua `os`.

```lua
local time = require("std.time")

local started = time.now_ms()
time.elapsed_ms(started)
time.seconds(1.5)
time.millis(250)
```

Cooldown helper:

```lua
local cooldown = time.cooldown(500)

if cooldown:ready() then
  cooldown:trigger()
end
```
