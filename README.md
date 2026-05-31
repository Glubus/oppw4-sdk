# OPPW4 SDK

Experimental SDK and modding runtime for **One Piece Pirate Warriors 4**.

This repository is moving away from language-specific prototype APIs toward a
registry-first SDK. Gameplay features are exposed as typed registry modules;
bridges such as JavaScript project those contracts into a language-friendly API.

The public surface is still allowed to break while the SDK is under active
development.

## What Is In This Repo

```text
apps/
  sdk-analyzer/          # standalone analyzer CLI, future base for LSP support
bridges/
  core/                  # mod manifests, registry contracts, events, reports
  js/                    # QuickJS bridge for JS mods
  js-analyzer/           # static JS analyzer library
crates/
  asm/                   # low-level patch/write helpers
  hooks/                 # hook helpers
  host/abi/              # plugin ABI
  host/core/             # plugin host
  rdb/                   # RDB helpers
  sdk/api/               # public plugin SDK API
examples/
  js/                    # recommended mod authoring path
  rust/                  # native plugin/native mod examples
plugins/
  moveset_patcher/       # official external plugin
sdk/
  plugins/               # official SDK service plugins
tools/                   # package, manifest, dump and data tools
oppw4-data/              # versioned local data snapshot
resources/               # reverse engineering resources and generated banks
docs/                    # architecture notes and reverse notes
```

## Current Direction

- Mods are loaded from `mod.toml` manifests, either as directories or zip files.
- JS mods are the preferred mod authoring path for gameplay scripting.
- JS can import SDK registry modules with `import { player } from "sdk"`.
- JS can split code with relative imports such as `import "./events/player.js"`.
- Runtime features are exposed by registry modules, not hardcoded in bridges.
- `sdk-analyzer` checks JS mods before runtime, similar in spirit to
  `cargo check`.
- Native Rust plugins remain available for services, low-level patchers and
  advanced integrations.

## Requirements

- Rust toolchain, currently developed with stable Rust.
- Git submodules initialized for the data snapshot and generated resources.
- Windows/MSVC target for producing game-loadable DLLs.
- PowerShell on Windows for the packaging script, or shell on Linux/WSL.

Initialize submodules:

```bash
git submodule update --init --recursive
```

## Build And Test

Run the normal workspace check:

```bash
cargo check --workspace
```

Run focused tests for the active registry, JS bridge and analyzer work:

```bash
cargo test -p sdk-bridge -p sdk-js-bridge -p sdk-js-analyzer -p oppw4-sdk-analyzer
```

On Linux, full `cargo test --workspace` can hit Windows-only plugin linkage for
some DLL plugins. Use the focused tests above when validating bridge/analyzer
work on Linux.

If your environment sets a read-only target dir or blocked `sccache`, use:

```bash
CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo check --workspace
```

## SDK Analyzer

The analyzer app lives in `apps/sdk-analyzer` and builds the `sdk-analyzer`
binary.

Run it through Cargo:

```bash
cargo run -q -p oppw4-sdk-analyzer -- check examples/js/player_event
```

Human output is the default:

```text
    Checking bridge-js mods
Finished sdk-analyzer: 1 file(s), 0 effect(s), 0 warning(s), 0 error(s)
```

Machine-readable JSON is available for tools and future LSP integration:

```bash
cargo run -q -p oppw4-sdk-analyzer -- check --json examples/js/player_event
```

Initialize the analyzer config shape for future bridge/plugin discovery:

```bash
cargo run -q -p oppw4-sdk-analyzer -- init bridge-js
cargo run -q -p oppw4-sdk-analyzer -- install bridge-js
```

Install the binary locally if you want `sdk-analyzer` directly in your shell:

```bash
cargo install --path apps/sdk-analyzer
sdk-analyzer check examples/js/player_event
```

The analyzer currently checks:

- `mod.toml` parsing and entry file existence;
- JS files under the provided root, including split/imported files;
- unresolved relative JS imports such as `import "./missing.js"`;
- registry method usage known by the analyzer;
- statically detected replacement effects;
- missing assets referenced by detected effects, with line/column output.
- missing `.bin` payloads passed to `replace_movesets(...)`.

Example failing check:

```bash
cargo run -q -p oppw4-sdk-analyzer -- check examples/js/analyzer-test
```

Expected style:

```text
    Checking bridge-js mods
error[asset_missing]: referenced asset does not exist: analyzer-test-body.g1t
  --> examples/js/analyzer-test/main.js:6:16
  |
6 |         body: "analyzer-test-body.g1t",
  |                ^^^^^^^^^^^^^^^^^^^^^^

note[effect]: luffy costume default texture.body with analyzer-test-body.g1t
  --> examples/js/analyzer-test/main.js

Failed sdk-analyzer: 2 file(s), 1 effect(s), 0 warning(s), 1 error(s)
```

Watch mode is available for continuous checks:

```bash
cargo run -q -p oppw4-sdk-analyzer -- check --watch examples/js/player_event
```

## JavaScript Mods

A JS mod is a folder with a `mod.toml` and an entry file:

```toml
[mod]
id = "player_event"
name = "Player Event JS Example"

[uses]
plugins = ["sdk_runtime"]

[entry]
file = "main.js"
```

Simple event mod:

```js
import { player } from "sdk";

player.on_character_changed((ctx) => {
    oppw4.trace(`player changed payload=${ctx.payloadJson}`);
});
```

Split code is supported by the bridge at runtime:

```js
// main.js
import "./events/player.js";
```

```js
// events/player.js
import { player } from "sdk";

player.on_character_changed((ctx) => {
    oppw4.trace(`active=${ctx.payload.activeCharacterIds.join(",")}`);
});
```

The JS bridge supports directory mods and zipped mods. Relative imports are
resolved inside the mod root and cannot escape with `..`.

## Registry Modules

Registry modules are typed contracts registered by plugins. Bridges project
those contracts into their own runtime.

Current runtime modules registered by `sdk_runtime` include:

- `sdk.player`
  - `player.active_characters()`
  - `player.on_character_changed(...)`
- `sdk.difficulty`
  - difficulty snapshot/applied events
- `sdk.rank`
  - rank related runtime events
- `sdk.rewards`
  - reward related runtime events

Other service plugins can expose modules and type extensions. For example,
data/moveset plugins can expose character data and extend `sdk.Character` with
patcher methods.

## Manifests

Validate a JS/native mod manifest:

```bash
cargo run -q -p mod-manifest-tool -- examples/js/player_event/mod.toml
```

Validate a plugin manifest:

```bash
cargo run -q -p plugin-manifest-tool -- plugins/moveset_patcher/plugin.toml
```

Runtime mod manifests use:

```toml
[mod]
id = "my_mod"
name = "My Mod"

[uses]
plugins = ["sdk_runtime"]

[entry]
file = "main.js"
```

The entry path must be a safe relative file path.

## Examples

Main examples:

- `examples/js/player_event`
  - listens to runtime player events.
- `examples/js/character_registry_probe`
  - probes registry character APIs.
- `examples/js/ace_moveset_registry`
  - demonstrates registry-based moveset replacement; provide `ace_moveset.bin`
    next to `main.js` for an analyzer-clean run.
- `examples/js/analyzer-test`
  - intentionally fails analyzer asset validation.
- `examples/rust/log_plugin`
  - native plugin example.
- `examples/rust/native_mod`
  - experimental native mod example.

See [examples/README.md](examples/README.md) for more notes.

## Packaging

Build the SDK release layout on Windows:

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-sdk.ps1
```

On Linux or WSL:

```bash
tools/package-sdk.sh
```

The package is written to:

```text
dist/oppw4-sdk/
```

The layout includes:

- loader proxy as `dinput8.dll`;
- SDK service plugins under `plugins/sdk/`;
- official external plugins in their plugin folders;
- required data snapshot under `oppw4-data/`.

The package scripts reject legacy Lua artifacts (`.lua`, `.luac`) in the output.

By default the package script builds the sibling loader workspace at
`../oppw4-modloader`. Override it when needed:

```bash
LOADER_ROOT=/path/to/oppw4-modloader tools/package-sdk.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-sdk.ps1 -LoaderRoot path\to\oppw4-modloader
```

## Data Snapshot

`oppw4-data` is consumed as a local, versioned data snapshot. Runtime should not
depend on a live website or mutable remote data.

Expected long-term flow:

```text
contributors -> wiki/API -> schema validation -> exported JSON snapshot -> SDK package
```

Validate data files with the scripts under `oppw4-data/scripts/`.

## Architecture Notes

- [Registry-first architecture](docs/architecture/registry-first.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture rules](docs/RULES.md)
- [API surfaces](docs/API-SURFACES.md)
- [Plugin API](docs/PLUGIN-DEVELOPMENT.md)
- [Data dumper concept](docs/DATA-DUMPER.md)
- [SDK analyzer architecture](docs/SDK-ANALYZER.md)

The public mdBook documentation is expected to live in the sibling
`docs-oppw4-prism` repository.

## Development Notes

- Keep bridges domain-neutral. Gameplay concepts belong in registry modules.
- Prefer typed registry schemas over ad hoc bridge globals.
- Keep runtime mod assets mod-relative and validate them before applying
  patches.
- Keep JS examples analyzer-clean unless the example is explicitly meant to fail.
- Keep package output free of legacy Lua files.

## Cleanup Backlog

These are intentionally not done yet, but should stay visible:

- Build the actual `sdk-analyzer lsp` server on top of the existing check
  engine.
- Define the stable analyzer plugin ABI for future drop-in `.dll` checks.
- Add Zed/Vscode extension adapters.
- Decide the SDK single-DLL bundling/package strategy.
- Continue reducing large historical runtime/tool files that are outside the
  current analyzer/bridge cleanup.

## Status

This SDK is experimental. The current priority is making the registry contracts,
JS bridge, standalone analyzer and packaging story reliable before locking a
public API.
