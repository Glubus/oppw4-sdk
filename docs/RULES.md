# Project Rules

## Boundaries

- `crates/host/core` owns host loading, capabilities, logs, manifests, and registry storage.
- `crates/sdk/api` owns the Rust authoring API and must stay language-agnostic.
- `bridges/core` owns language runtime contracts.
- Language bridges live under `bridges/`, such as `bridges/js`.
- Official plugins register modules/events/mutations through the registry and do not call language bridges directly.
- SDK-shipped DLL plugins live under `sdk/plugins`.
- Gameplay plugins built on top of the SDK live under `plugins`.

## Registry

- Plugins register registry modules with stable provider and module names.
- Mods request plugins through manifest dependencies.
- Bridges install only modules selected by registry metadata.
- Domain APIs must not be hardcoded in bridges.

## Mod Paths

- JavaScript through QuickJS is the recommended mod runtime.
- Native Rust mods are experimental and advanced.
- Adding another language must add a bridge crate, not host/core logic.
