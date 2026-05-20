local character = require("std.character")
local log = require("std.log")
local moveset_patcher = require("moveset_patcher")

local young_garp_moveset = moveset_patcher.moveset({
  section_count = 1,
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

character.find("garp_yng"):replace_movesets(young_garp_moveset)
log.info("registered LinkData moveset patch for garp_yng")
