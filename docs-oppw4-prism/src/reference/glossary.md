# Glossary

`loader`

The native bootstrap layer loaded through the game process. It exposes primitives to SDK core.

`SDK core`

The plugin host layer: manifests, capabilities, Lua runtime, service routing, mod discovery, logs, configs.

`SDK service`

An official plugin that owns a game-specific system such as runtime probes, LinkData, or RDB.

`data bank`

The `oppw4-data` repository containing character metadata, costumes, assets, body parts, movesets, and generated indexes.

`mod`

A user package under `mods/` that can contain Lua and assets.

`plugin`

A native DLL package under `plugins/` with a `plugin.toml`.

`capability`

A manifest-declared permission required before a plugin can call certain SDK operations.
