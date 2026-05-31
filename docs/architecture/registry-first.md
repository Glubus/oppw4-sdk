# Registry-First Architecture

The SDK is split into three layers.

## Host and SDK

`crates/host/abi` defines the stable FFI table. `crates/host/core` owns plugin
loading, host services, capability checks, and registry storage. `crates/sdk/api`
is the Rust API used by official plugins and experimental native Rust mods.

These crates must stay language-agnostic. They can talk about registry modules,
events, mutations, files, memory, and capabilities. They must not depend on any
language bridge.

The current ABI still exposes some game/runtime-domain functions directly,
notably `game_status`, `active_character`, and their provider registration
hooks. Treat this as legacy shape, not the desired long-term boundary. These
concepts belong to the runtime service layer and should eventually move behind
registry modules/capabilities provided by `sdk_runtime`, with `plugin-sdk`
keeping ergonomic wrappers on top. Do not expand the core ABI with more
game-specific fields unless there is a short-term migration reason.

## Bridge Layer

`bridges/core` defines the language-runtime contract: load a mod, expose
registry modules, dispatch events, and return logs/errors/mutations.
`bridges/js` is the QuickJS implementation of that contract.

Adding another language should create a new bridge crate and should not require
changes in `crates/host/core` or `crates/sdk/api`.

## Official Plugins

SDK plugins live under `sdk/plugins`. Official SDK plugins such as
`sdk_runtime`, `sdk_data`, and `sdk_rdb` register capabilities and registry
modules. Runtime code publishes events and applies validated mutations, but does
not know which language produced the mutation.

## Mod Paths

JavaScript is the recommended mod runtime. Rust native mods are experimental and
reserved for authors comfortable with native DLL ABI constraints.
