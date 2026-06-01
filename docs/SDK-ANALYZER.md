# SDKT Architecture

`sdkt` is the standalone development toolkit for SDK mods. It stays separate
from the JS bridge runtime so it can be reused by editors and future language
servers.

## Current CLI

```bash
sdkt check examples/js/player_event
sdkt check --json examples/js/player_event
sdkt check --watch examples/js/player_event
sdkt config --game-path d:/idk/oppw4
sdkt config show
sdkt config get game-path
sdkt init
sdkt install bridge-js
sdkt run
sdkt test
sdkt package --ignore-warnings
```

`bridge-js` is currently built in. The `init` and `install` commands create the
first on-disk shape for future bridge/plugin discovery:

```text
.sdkt/
  config.toml
  bridges/
    bridge-js.toml
```

## Responsibilities

- Load SDK registry contracts available to a mod.
- Analyze source files without executing mod code.
- Report diagnostics in a cargo-check-like human format.
- Emit structured JSON for tools and future LSP processes.
- Validate mod-local assets before runtime.
- Validate bridge-local source graph issues such as missing relative imports.

## Future LSP Shape

The intended layering is:

```text
editor extension -> sdkt lsp -> sdkt check engine -> bridge analyzers
```

The LSP should not duplicate analyzer logic. It should keep projects warm, run
incremental checks, and translate diagnostics/completions/actions into LSP.

## Future Plugin Shape

Analyzer bridge plugins should be discoverable next to the analyzer:

```text
.sdkt/
  plugins/
    my-check.dll
```

Those plugins should receive a stable analyzer ABI: mod root, manifest, registry
snapshot, source files and configured bridge. They should return structured
diagnostics with source spans.

This is not implemented yet. The current `install bridge-js` command records the
built-in bridge in the same shape so the CLI does not need to change later.
