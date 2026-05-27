# Crates

Core Rust crates are grouped by ownership.

- `host`: ABI and host loader/runtime implementation.
- `sdk`: Rust authoring API for plugins and experimental native Rust mods.
- `asm`, `hooks`, `rdb`: shared low-level/support crates.

Language runtimes do not belong here; they live under `bridges/`.
