# Build And Test

Run the normal SDK test set:

```bash
cargo test --workspace
```

For Windows runtime compatibility checks from Linux/WSL, use the Windows GNU target:

```bash
cargo test --workspace --target x86_64-pc-windows-gnu --no-run
```

Some official plugin tests may not run natively on Linux because they link to Windows APIs such as `kernel32` or `user32`. In that case, the useful verification is usually:

```bash
cargo test -p <package> --target x86_64-pc-windows-gnu --no-run
```

Validate manifests:

```bash
cargo run -p plugin-manifest-tool -- official_plugins/fx_director/plugin.toml
cargo run -p mod-manifest-tool -- path/to/mod.toml
```

Build this documentation:

```bash
mdbook build docs-oppw4-prism
```

The generated HTML lives in `docs-oppw4-prism/book/` and is ignored by Git.
