# Architecture Decisions

## Loader stays boring

The loader is only a bootstrap and primitive provider. This prevents every new feature from growing the proxy ABI.

## SDK core orchestrates

SDK core owns lifecycle, capability checks, Lua runtime, mod discovery, logs, configs, and service routing. It should not know how a character costume, LinkData row, or RDB archive works.

## Game systems are services

Runtime probes, LinkData, and RDB are official SDK service plugins because they are game-specific but shared by multiple features.

## Data lives outside code

`oppw4-data` is mandatory runtime data and an editable submodule. This lets the community improve character metadata without recompiling SDK code.

## Lua std is modular

Each `std.*` module lives under `crates/lua-runtime/src/std_plugins/<module>/`. `lua-runtime` owns sandboxing and `require`; std modules own their own API behavior.

## Config schema is registered

Plugins can register config schemas/defaults through the SDK ABI. Tools can discover config shape without hardcoding each plugin.

## ABI uses `struct_size`

`struct_size` allows append-only ABI evolution. New fields go at the end, older readers can detect missing fields, and newer hosts can add optional callbacks without breaking older plugins.
