# Setup

This repository is the SDK workspace. The loader is a separate project that builds `dinput8.dll`.

Required local pieces:

- Rust toolchain;
- Windows GNU target for runtime builds;
- `oppw4-data` submodule;
- optional `mdbook` for documentation.

Initialize data:

```bash
git submodule update --init --recursive
```

Add the Windows target when needed:

```bash
rustup target add x86_64-pc-windows-gnu
```

The SDK is designed to run in the game on Windows. Linux/WSL is useful for editing, checking, and building Windows-target artifacts, but native execution of Windows hook/plugin crates is not the target.

Documentation build tool:

```bash
cargo install mdbook
```

Mermaid diagrams are not enabled by default. Install and configure `mdbook-mermaid` later if rendered diagrams become necessary.
