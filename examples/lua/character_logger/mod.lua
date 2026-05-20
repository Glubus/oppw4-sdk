local character = require("std.character")
local log = require("std.log")
local mod = require("std.mod")

local current = mod.current()
local law = character.find("law")

log.info(current.id .. " loaded")
log.info("found character " .. law.canonical .. " runtime_id=" .. law.ids.runtime)
