# std.path

`std.path` manipulates mod and asset path strings. It never reads files.

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
