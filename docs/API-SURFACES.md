# API Surfaces

## Host ABI

`crates/host/abi` exposes the C-compatible host table and FFI structs. It should
contain only stable primitives, callbacks, and registry/module descriptors.

## Rust SDK

`crates/sdk/api` exposes:

- `Plugin` and `export_plugin!` for native plugins.
- `PluginFeature` and feature helpers for capabilities.
- `RegistryModuleFeature` and `registry_module!` for registry modules.
- `RustMod` and `export_rust_mod!` for experimental native Rust mods.

## Bridge Runtime

`bridges/core` exposes:

- `RuntimeAdapter` for language runtimes.
- `BridgeRegistry` for loading mods and dispatching events.
- `EventEnvelope` and `MutationEnvelope` for bridge I/O.
- `RegistryModuleDescriptor` for language-independent module installation.

## JavaScript

`bridges/js` exposes the initial QuickJS runtime:

```js
oppw4.on("sdk.runtime.event", (ctx) => {
    oppw4.trace(ctx.payloadJson);
});
```

Registry modules are discovered through the registry and installed by the bridge.

Typed registry events can also project event-specific fields directly on `ctx`:

```js
import { player } from "sdk";

player.on_character_changed((ctx) => {
    oppw4.trace(ctx.current_character?.id ?? "none");
});
```
