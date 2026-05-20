local moveset_patcher = require("moveset_patcher")
local character = require("std.character")

local my_movesets = moveset_patcher.moveset({
  payload_file = "garp_moveset_readable.json",
})

character.find("garp"):replace_movesets(my_movesets)
