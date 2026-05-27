# OPPW4 SDK

This workspace is the experimental OPPW4 SDK split from the original patcher
into a registry-first modding platform.

The SDK owns the modding platform:

- SDK core orchestration and plugin loading;
- host ABI/API crates;
- language bridge infrastructure;
- typed registry modules and type extensions;
- OPPW4 character/data APIs backed by the `oppw4-data` snapshot;
- RDB and LinkData helpers;
- SDK service plugins, external plugins, examples, and modder documentation.

This is still experimental and unpublished. Compatibility with older prototype
Lua APIs is not a goal; the SDK should stay clean and registry-first while the
public surface is still allowed to break.

## Layout

```text
crates/
  host/
    abi/          # current package: plugin-abi
    core/         # current package: plugin-host
  sdk/
    api/          # current package: plugin-sdk
  hooks/
  asm/
  rdb/
bridges/
  core/           # runtime registry, manifests, events, mutations
  js/             # QuickJS bridge
sdk/
  plugins/        # SDK service plugins packaged under plugins/sdk/
plugins/
  moveset_patcher/ # external official plugin
oppw4-data/       # local data snapshot consumed by sdk_data
examples/
docs/
```

Clone or update the data snapshot before building packages or working on
character data:

```bash
git submodule update --init --recursive
```

## Build

```powershell
cargo test --workspace
```

The SDK services are packaged under `plugins/sdk/`. External official plugins
such as `moveset_patcher` are packaged in their own plugin folders.

Validate an external plugin manifest:

```powershell
cargo run -p plugin-manifest-tool -- plugins/moveset_patcher/plugin.toml
```

Validate a runtime mod manifest:

```powershell
cargo run -p mod-manifest-tool -- path/to/mod.toml
```

Build the SDK release layout:

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-sdk.ps1
```

On Linux or WSL:

```bash
tools/package-sdk.sh
```

The package is written under `dist/oppw4-sdk/` with the loader proxy as
`dinput8.dll`, SDK services in `plugins/sdk/`, external official plugins in
their own plugin folders, and the mandatory data repository under `oppw4-data/`.
Active mods are not copied into plugin folders; runtime mods belong under the
game-level `mods/` directory.

By default the package script also builds the sibling loader workspace at
`../oppw4-modloader`. Use `LOADER_ROOT=/path/to/oppw4-modloader` on shell or
`-LoaderRoot path\to\oppw4-modloader` on PowerShell when the loader repository
is elsewhere.

Examples are documented in [examples/README.md](examples/README.md).

## Registry-First Runtime

The runtime is built around typed registry modules instead of language-specific
standard libraries. A plugin can expose a module such as `sdk.character` or add
methods to another module's type through registry type extensions. Language
bridges project those typed contracts into their own syntax, but the bridge does
not own gameplay concepts.

For example, `sdk_data` exposes `sdk.Character` values, and
`moveset_patcher` extends `sdk.Character` with `replace_movesets(...)`. A JS mod
can therefore stay small:

```js
import { character } from "sdk";

const zoro = character.find("zoro");
zoro.replace_movesets("zoro_new_world_moveset.bin");
```

The VM passes only the declarative request. Rust validates the mod-relative
payload path, reads the `.bin`, and applies the LinkData replacement.

## Data Direction

`oppw4-data` is currently consumed as a local snapshot. The long-term direction
is a collaborative, schema-validated data site that acts like a wiki for reverse
engineering evidence and game data. The game runtime should never depend on the
live site; it should consume a versioned exported snapshot.

Expected flow:

```text
contributors -> wiki/API -> schema validation -> versioned JSON snapshot -> SDK package
```

Versioning should treat normal data additions and corrections as `1.x.y`
changes. A major version bump is reserved for breaking snapshot contracts, which
should be rare while the SDK remains experimental.

## Architecture Docs

The public mdBook documentation now lives in the sibling `docs-oppw4-prism`
repository.

- [SDK foundation RFC](docs/RFC-0001-sdk-foundation.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture rules](docs/RULES.md)
- [API surfaces](docs/API-SURFACES.md)
- [Plugin API design](docs/PLUGIN-API.md)
- [Plugin development](docs/PLUGIN-DEVELOPMENT.md)
- [SDK data dumper concept](docs/DATA-DUMPER.md)
