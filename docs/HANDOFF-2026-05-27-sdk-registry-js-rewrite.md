# Handoff: SDK Registry/JS Rewrite

Date: 2026-05-27

## What Changed

- Removed the active Lua runtime/bridge path from the SDK rewrite.
- Moved language bridges out of `crates/`:
  - `bridges/core` keeps the language-neutral bridge registry.
  - `bridges/js` owns QuickJS projection/execution.
- Moved host/sdk crates into clearer ownership folders:
  - `crates/host/abi`
  - `crates/host/core`
  - `crates/sdk/api`
- Moved official SDK plugins under `sdk/plugins/`.
- Moved standalone plugins under `plugins/`.
- Added/kept docs for the registry-first architecture.
- Split large registry/loader modules into smaller files.
- Renamed rank diagnostics from `helper_probe` to `rank/diagnostics/helper_hooks`.
- Kept rank `count` terminology generic, not `killcount`, so it can later represent a computed score.

## Registry/Bridge Direction

- There is one global registry. There is no separate "JS registry".
- Bridges project the global registry into their language.
- JS should not hardcode gameplay APIs in the bridge.
- The JS bridge only owns language mechanics:
  - QuickJS runtime lifecycle
  - module loading/imports
  - handler registration
  - registry projection
  - invocation routing

## JS Registry Projection

The JS bridge now supports typed registry module projection.

Example:

```js
import { character } from "sdk";

const zoro = character.find("zoro");
```

The flow is:

```text
JS call
-> __oppw4_registry_invoke("sdk.character.find", argsJson)
-> JS bridge generic router
-> registry module invoke handler
-> resultJson
-> JS value
```

The bridge can project registry schemas into namespace modules such as:

```js
import { character } from "sdk";
```

and also exposes metadata through:

```js
oppw4.registry.modules();
oppw4.registry.module("sdk.character");
oppw4.registry.has("sdk.character");
```

## Typed Registry Schema

`sdk-bridge` now has schema descriptors:

- `RegistryModuleSchema`
- `RegistryFunctionDescriptor`
- `RegistryTypeDescriptor`
- `RegistryTypeRef`

These let a module describe:

- namespace, e.g. `sdk`
- import name, e.g. `character`
- constructibility
- functions
- parameters
- return types
- exposed data types

## Current Validation

Passing:

```bash
RUSTC_WRAPPER= rtk cargo test -p sdk-js-bridge
RUSTC_WRAPPER= rtk cargo check --workspace
```

Known limitation:

```bash
cargo test -p oppw4-sdk-runtime-plugin
```

still fails on Linux when linking Windows hook dependencies such as `user32`.
This is not new; tests need to be split between pure logic and Windows/hook integration.

## Important State

- `sdk-js-bridge` has a working in-process test for:
  - typed schema projection
  - `import { character } from "sdk"`
  - generic invocation to Rust
  - JSON argument/result transport
- The current invocation path works for modules passed directly to the JS bridge.
- The next step is to expose this same invoke path through the host/plugin ABI, so real SDK plugins can register callable functions dynamically.

## What Remains

1. Add registry invoke support to the host/plugin ABI.
   - A plugin should register module schema + invoke callback.
   - Host should store both in the global registry.
   - Bridges should receive descriptors and invocation routing from the registry.

2. Build the first real SDK registry module.
   - Good target: `sdk.character`.
   - Desired JS:
     ```js
     import { character } from "sdk";
     const zoro = character.find("zoro");
     ```
   - `Character` should be a returned typed object, not a public constructor unless the schema says so.

3. Add a player/event module next.
   - Desired JS shape:
     ```js
     import { character, player } from "sdk";

     player.on_character_changes((ctx) => {
       const zoro = character.find("zoro");
       if (ctx.active_characters().some((active) => active.id === zoro.id)) {
         // ...
       }
     });
     ```

4. Keep `count` generic for rank/score.
   - Do not rename it to `killcount`.
   - Future score can include territories, commanders, side objectives, etc.
   - JS should eventually be able to contribute score logic through registry modules.

5. Continue cleaning runtime rank boundaries.
   - Diagnostics/reverse should stay under `diagnostics`.
   - Gameplay patches should be separate from probes/hooks.
   - Stable SDK surfaces should be registry modules.

6. Improve QuickJS module ergonomics.
   - Better JS stack traces/errors.
   - Possibly source names/source maps.
   - Controlled imports only.
   - Unload lifecycle.

7. Split runtime tests.
   - Pure Rust tests should run on Linux.
   - Windows hook/link tests should be gated separately.

## Architecture Rule To Keep

Do not add gameplay hardcoding to the JS bridge.

Correct:

```text
global registry describes sdk.character
JS bridge projects sdk.character
JS calls character.find
registry routes the call
provider returns JSON/value
```

Wrong:

```text
JS bridge manually implements character.find
JS bridge knows sdk runtime internals
language bridge owns gameplay concepts
```
