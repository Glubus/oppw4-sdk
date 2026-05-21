# Lua Standard Library

The SDK Lua standard library is SDK-owned and installed before mod scripts run.
It is split from the Lua runtime implementation so runtime code stays focused on
sandboxing, `require`, mod context, and module registration.

Implementation layout:

```text
crates/lua-runtime/src/runtime.rs        # sandbox/require/runner orchestration
crates/lua-runtime/src/runtime/          # Lua runtime internals
crates/lua-runtime/src/std_plugins.rs    # installs SDK std modules
crates/lua-runtime/src/std_plugins/      # one folder per std module
crates/lua-runtime/src/std_plugins/math/mod.rs
crates/lua-runtime/src/std_plugins/buffer/{mod,writer,reader,bytes,tests}.rs
```

Standard modules are exposed through both `require("std.<name>")` and
`std.<name>`. The legacy global `character` remains a transition alias for
`std.character`.

## Runtime Boundary

`lua-runtime` owns:

- Lua VM creation and sandboxing;
- SDK-controlled `require`;
- current mod globals;
- mod script execution;
- `register_std_module` and plugin module registration.

`std_plugins` owns:

- SDK standard modules;
- module-specific tests;
- standard handle extension registries such as `std.character`.

Each `std.*` module must live in its own folder, even when the first
implementation is small. Larger modules should split behavior into focused
files early instead of growing a monolithic `mod.rs`.

Std modules must not expose arbitrary filesystem, process, debug, or package
access. `os`, `io`, `debug`, and global `package` remain hidden from mod
scripts.

## Current Modules

### `std.character`

Character bank access and SDK-approved character handle extensions.

```lua
local character = require("std.character")
local garp = character.find("garp")
```

### `std.files`

Safe reads from the current mod root or current mod zip.

```lua
local files = require("std.files")
local text = files.read_text("config.lua")
local bytes = files.read_bytes("payload.bin")
```

### `std.log`

Mod-scoped log entries routed through SDK core.

```lua
local log = require("std.log")
log.info("loaded")
log.warn("fallback used")
```

### `std.mod`

Current mod metadata and source information.

```lua
local mod = require("std.mod")
local current = mod.current()
print(current.id, current.is_zip)
```

### `std.math`

Small deterministic numeric helpers.

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

### `std.path`

Path string helpers for mod and asset paths. This module never reads files.

```lua
local path = require("std.path")

path.join("assets", "garp", "body.g1t")
path.normalize_slashes("assets\\garp\\body.g1t")
path.basename("assets/garp/body.g1t")
path.extension("assets/garp/body.g1t")
path.stem("assets/garp/body.g1t")
path.parent("assets/garp/body.g1t")
path.is_safe_relative("assets/garp/body.g1t")
```

### `std.time`

Monotonic runtime time helpers and simple cooldowns. This module does not expose
Lua `os`.

```lua
local time = require("std.time")

local started = time.now_ms()
time.elapsed_ms(started)
time.seconds(1.5)
time.millis(250)

local cooldown = time.cooldown(500)
if cooldown:ready() then
  cooldown:trigger()
end
```

### `std.collections`

Strict utility collections for scripts that need predictable behavior beyond raw
Lua tables.

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

local history = collections.ring_buffer(60)
history:push(sample)
history:last()
history:values()
```

`map` accepts only string, integer, and boolean keys. `ring_buffer` returns
values from oldest to newest.

### `std.buffer`

Binary payload reader/writer helpers for scripts that need to build or inspect
little-endian data without string byte hacks.

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

The reader accepts a Lua string or a byte table. Numeric writers validate the
target range before appending data.

## Deferred Modules

- `std.string`: deferred because Lua already provides `string.*`.
- `std.primitives`: deferred until a concrete typed payload API needs it.
- `std.memory`: should stay behind SDK/plugin capabilities, not general mod std.
