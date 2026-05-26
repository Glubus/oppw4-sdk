# SDK Plugin API

## Status

This design targets `v0.0.1-experimental`.

The SDK is not public and has no compatibility promise yet. Breaking changes are
allowed when they make the SDK easier to use, easier to bridge, or easier to
maintain. The only behavior that must be preserved during this reset is the
current `sdk.rdb.patcher` integration inside `sdk.rdb`.

## Goals

The plugin API should make the easy path obvious:

- a native Rust plugin should not need to write raw `unsafe extern "system"`
  callbacks for normal SDK operations;
- public functions should stay short, predictable, and easy to call from plugin
  code;
- common registration flow should be one or two lines per feature;
- host errors, capability checks, logging, and manifest mismatches should be
  reported through one consistent path;
- SDK-owned services such as `sdk.rdb`, `sdk.runtime`, `sdk.linkdata`,
  `sdk.debug`, and `sdk.overlay` should use the same feature model as future
  third-party plugins;
- public plugin identity should stay simple for authors while the SDK rejects
  duplicate active ids;
- the Rust model should be bridgeable so a future `sdk_bridge` can support
  runtimes such as `sdk_python`.

Non-goals for this design:

- preserving the old `skin_patcher` or `fx_director` standalone plugin shape;
- hiding the SDK service architecture behind the loader;
- supporting arbitrary scripting runtimes before the Rust feature model is
  stable;
- making the FFI ABI larger unless a bridge or runtime feature actually needs
  it.

## Current Shape

The current low-level plugin entrypoint is already small:

```rust
pub trait Plugin {
    const ID: &'static str;

    fn log_policy() -> LogPolicy {
        LogPolicy::HOST
    }

    fn init(context: PluginContext<'_>) -> PluginResult<()>;
}
```

`export_plugin!(Type)` exports `oppw4_plugin_init`, validates
`Oppw4PluginApi`, builds a `PluginContext`, and calls `Plugin::init`.

That shape is good enough for the DLL boundary. The problem is inside
`Plugin::init`: feature plugins and SDK services still manually call host
services, build provider structs, keep global callback state, translate host
return codes, and repeat capability assumptions.

Examples in the current tree:

- `sdk.rdb` registers the RDB service, generic virtual file provider, dispatch
  callbacks, patch providers, handle storage, and `sdk.rdb.patcher` by hand.
- `sdk.rdb.patcher` is now a library crate under `official_plugins/sdk/rdb/patcher`
  and is initialized by `sdk.rdb`.
- the former standalone `skin_patcher` has moved into `sdk.rdb.patcher`.
- the former standalone `fx_director` runtime code has moved into
  `sdk.runtime`.

That migration is the right direction: official behavior should become SDK
service features, not separate public plugin entrypoints.

## Target Model

The new model has two layers.

Layer 1 is the native plugin boundary:

- `Plugin` remains the DLL-level trait.
- `export_plugin!` remains the native export macro.
- `Oppw4PluginApi` remains the append-only ABI table.
- plugin ids stay short and readable; the SDK rejects duplicate active plugin
  ids.

Layer 2 is the feature registration layer:

- `PluginFeature` is a reusable unit that can be installed by a plugin or SDK
  service.
- typed feature traits model common SDK integrations.
- `PluginRegistrar` owns the registration flow and keeps boilerplate out of
  plugin code.

The intended shape is:

```rust
use plugin_sdk::{Plugin, PluginContext, PluginResult};

struct SdkRdb;

impl Plugin for SdkRdb {
    const ID: &'static str = "sdk_rdb";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context
            .registrar()
            .add(sdk_rdb::service())?
            .add(sdk_rdb_patcher::feature())?
            .finish()
    }
}
```

Normal plugin authors should spend their time writing feature behavior, not
registration glue.

Macros should exist for the common path. Traits remain the real API; macros are
only boilerplate reducers that expand to normal trait implementations.

```rust
plugin_sdk::sdk_plugin! {
    id = "sdk_rdb",
    features = [
        sdk_rdb::service(),
        sdk_rdb_patcher::feature(),
    ],
}
```

The macro path should be optional. Any plugin that needs unusual initialization
can still implement `Plugin` manually.

## Plugin Identity

Plugin ids should be simple.

Authors should write short ids such as `zoro_elbaf`, `rdb_patcher`, or
`moveset_patcher`. Long reverse-DNS or registry-prefixed ids are bad for modders
and bad for Lua/module ergonomics.

The SDK still needs a collision model. A plugin is not a user mod: it is a
technical SDK integration installed under `plugins/`. It is acceptable, and
preferable, to require active plugin ids to be unique.

Identity layers:

- `plugin.id`: short stable id chosen by the plugin author.
- `plugin.name`: human-facing display name.
- package owner/source: registry metadata, local folder metadata, or bridge
  metadata.

Recommended identity fields:

```toml
[plugin]
id = "zoro_elbaf"
name = "Zoro Elbaf"
version = "0.1.0"
entry = "zoro_elbaf.dll"
```

Rules:

- `plugin.id` is short, stable, lowercase-normalized, and author-facing.
- `plugin.name` is human-facing and may collide.
- registry/package metadata owns author/source identity when the plugin is
  distributed.
- folder names are packaging details and must not be the final authority.
- official SDK services may keep dotted reserved ids such as `sdk.rdb`.
- bridge runtimes may keep reserved ids such as `sdk_bridge` and `sdk_python`.
- plugin dependencies reference the simple id.
- Lua module names should default to the simple id unless the SDK grants a
  reserved public module name.

Example collision:

- Alice publishes `id = "zoro_elbaf"`.
- Bob publishes another `id = "zoro_elbaf"`.
- the registry or package validation rejects the second plugin id, or asks the
  author to choose another id.
- a local install with both active plugins is invalid.
- one author can rename to `zoro_elbaf_battle` or another distinct technical id.

The registry should also support aliases and renames:

- aliases map old simple ids to the current simple id;
- the SDK logs a warning when an alias is used;
- aliases are registry data, not plugin-authored claims;
- two active plugins cannot have the same simple id.

## Core Traits

The exact Rust API can change during implementation, but the ownership model
should stay close to this.

```rust
pub trait PluginFeature {
    fn id(&self) -> &'static str;

    fn required_capabilities(&self) -> &'static [&'static str] {
        &[]
    }

    fn install(&self, registrar: &mut PluginRegistrar<'_>) -> PluginResult<()>;
}
```

`PluginFeature::id` is a local feature id, not necessarily a manifest plugin id.
For example, `sdk.rdb.patcher` is a feature inside the `sdk_rdb` plugin.

```rust
pub struct PluginRegistrar<'api> {
    context: PluginContext<'api>,
}

impl<'api> PluginRegistrar<'api> {
    pub fn add<F>(&mut self, feature: F) -> PluginResult<&mut Self>
    where
        F: PluginFeature,
    {
        self.require_all(feature.required_capabilities())?;
        feature.install(self)?;
        self.log_feature_installed(feature.id());
        Ok(self)
    }

    pub fn finish(self) -> PluginResult<()> {
        Ok(())
    }
}
```

The registrar is allowed to know how to call host services. Features should not
need to hold `Oppw4PluginApi` or `host_context` directly.

## Function Ergonomics

Public SDK functions should be boring to call.

Rules:

- prefer short verbs such as `add`, `install`, `register`, `emit`, `read`, and
  `patch`;
- keep common calls to one line;
- use small typed request structs when a function would need more than three
  meaningful arguments;
- use builders only for optional settings, not for required data;
- return `PluginResult<T>` instead of raw host error codes;
- keep unsafe functions behind safe feature wrappers whenever practical.

Good:

```rust
context.registrar().add(rank_plugin::feature())?.finish()
rank.override_thresholds(thresholds)?
signals.emit("sdk.runtime.rank.changed", payload)?
```

Avoid:

```rust
unsafe_register_rank_patch_provider(
    host_context,
    plugin_id,
    capability_name,
    provider_context,
    callback,
    flags,
)
```

## Host Service API Style

Every `host.*` surface should feel like the same SDK, even when it talks to very
different runtime systems.

Rules:

- service getters live on `HostApi` and `OwnedHostApi`, for example
  `host.rank()`, `host.rdb()`, `host.lua()`, and `host.signals()`;
- service methods use verbs first: `register_*`, `set_*`, `emit_*`,
  `replace_*`, `insert_*`, `remove_*`, `read`, `write`, `scan`;
- required data is passed directly to the constructor or method;
- optional data is configured with builder methods;
- raw ABI structs and callbacks are accepted only at the low-level escape hatch;
- high-level functions return `PluginResult<T>`;
- API types should be serializable when they cross service boundaries;
- public names describe modder intent, not reverse-engineering labels.

Current cleanup targets:

| Service | Current role | Cleanup direction |
| --- | --- | --- |
| `host.rank()` | rank cap and count threshold runtime commands | keep the public model slot-based and condition-based |
| `host.rdb()` | RDB virtual provider registration | expose pointer-free virtual/patch traits above callbacks |
| `host.linkdata()` | typed LinkData entry and row edits | keep row targets typed and add request builders where calls grow |
| `host.lua()` | Lua module registration | prefer `LuaModuleFeature` / `lua_module!` for normal plugins |
| `host.files()` | virtual file providers | move most plugins to safe provider traits |
| `host.signals()` | service-to-service command bus | add typed payload helpers where commands stabilize |
| `host.memory()` | process memory primitives | keep capability-gated and avoid exposing it to bridges by default |
| `host.configs()` | config schema registration | keep `ConfigFeature` as the common path |

### Rank

Rank rules are slot-based. The slot is the rank produced by the game reward
flow. There is no separate target field.

Known reward slots:

| Slot | Alias | Rank |
| --- | --- | --- |
| `0` | `"d"` | D |
| `1` | `"c"` | C |
| `2` | `"b"` | B |
| `3` | `"a"` | A |
| `4` | `"s"` | S |
| `5` | `"s_plus"`, `"s+"` | S+ |

The Rust API accepts numeric slots or aliases and normalizes known values:

```rust
host.rank().set_rank_cap(
    RankCapRule::enable()
        .slots([4, 5])
        .condition(Option::<RankCondition>::None),
)?;
host.rank().set_rank_cap(
    RankCapRule::disable()
        .slot("d")
        .condition(Option::<RankCondition>::None),
)?;
host.rank().set_rank_cap(RankCapRule::enable().slot(RankSlot::s_plus()))?;
```

`condition` is explicit. `None` means unconditional; `All` and `Any` support
multiple predicates:

```rust
host.rank().set_rank_cap(
    RankCapRule::enable()
        .slots([4, 5])
        .all([
            RankCondition::active_character("zoro"),
            RankCondition::flag("crew.elbaph", true),
        ]),
)?;
```

The equivalent Lua shape should stay simple and permissive:

```lua
rank:slot({4, 5}):condition(nil):enable()
rank:slot({"s", "s_plus"}):condition(nil):enable()
rank:slot("d"):condition(nil):disable()

rank:slot({4, 5})
  :condition(all(
    character:active_character("zoro"),
    flag("crew.elbaph", true)
  ))
  :enable()
```

Lua condition behavior:

- `condition(nil)` means unconditional;
- `condition(all(...))` means every predicate must match;
- `condition(any(...))` means at least one predicate must match;
- unknown/custom conditions are passed through for future runtime hooks.

Rank cap effects:

| Effect | Meaning |
| --- | --- |
| `Enable` | selected slots are reachable when the condition matches |
| `Disable` | selected slots are blocked when the condition matches |
| `KeepDefault` | leave the game behavior unchanged for this rule |
| `Custom(id)` | pass an effect id to a future or plugin-owned hook |

The implementation may only support a subset of rules at first. That is fine:
the API must still be able to describe the rule clearly, log unsupported runtime
combinations clearly, and keep the public shape stable while hooks are added.

## Macro Layer

Macros are part of the ergonomics story. They should remove repeated syntax,
not hide ownership or capability rules.

The SDK should provide macros in two phases:

- `macro_rules!` declarative macros first for common zero-magic cases;
- procedural macros later, only after the trait layer and `sdk.rdb.patcher`
  migration prove that attributes or derives remove real boilerplate that
  declarative macros cannot express cleanly.

The `v0.0.1-experimental` implementation should use `macro_rules!` only. Do
not add a `plugin-sdk-macros` proc-macro crate until the high-level trait model
has survived the first service migration.

Recommended first macros:

```rust
sdk_plugin! {
    id = "my_plugin",
    name = "My Plugin",
    features = [
        lua_feature(),
        config_feature(),
        rdb_feature(),
    ],
}
```

Expands to:

- a zero-sized plugin type;
- an implementation of `Plugin`;
- one `PluginRegistrar` flow;
- the existing `export_plugin!` export.
- a compile-time constant for the plugin id that feature macros can reuse.

```rust
lua_module! {
    plugin = PLUGIN_ID,
    module = "sdk.rdb.patcher",
    register = register_lua,
}
```

Expands to a `LuaModuleFeature` implementation backed by the provided register
function.

```rust
config_schema! {
    name = "config",
    toml = include_str!("../config.default.toml"),
}
```

Expands to a `ConfigFeature`.

```rust
rdb_patch_feature! {
    id = "sdk.rdb.patcher",
    patch_read = patch_read,
}
```

Expands to an `RdbPatchFeature` adapter. The generated adapter owns the native
callback trampoline if the implementation needs to cross the ABI boundary.

Recommended later procedural macros:

```rust
#[derive(PluginFeatureGroup)]
#[plugin_feature_group(id = "sdk.rdb.patcher")]
struct RdbPatcherFeature {
    lua: RdbPatcherLua,
    rdb: RdbPatcherProvider,
}
```

Expands to a `PluginFeature` that installs each field in order.

```rust
#[sdk_lua_module(module = "sdk.rdb.patcher")]
fn register_lua(lua: LuaModuleRegistrar<'_>) -> PluginResult<()> {
    Ok(())
}
```

Expands to the matching `LuaModuleFeature` wrapper and native Lua register
trampoline if the final runtime API still needs one.

```rust
#[sdk_rdb_patch(id = "sdk.rdb.patcher")]
fn patch_read(request: RdbPatchRead<'_>) -> PluginResult<RdbPatchResult> {
    Ok(RdbPatchResult::unchanged())
}
```

Expands to a safe `RdbPatchFeature` implementation and keeps pointer handling in
SDK-owned code.

Macro rules:

- every macro must expand to the same public traits that manual code can use;
- `v0.0.1-experimental` macros should be `macro_rules!` macros;
- procedural macros require a separate design pass after `sdk.rdb.patcher` has
  migrated to the trait/registrar API;
- plugin id values passed to macros should be simple SDK ids, not package
  registry paths;
- macro output must preserve normal `PluginResult` error flow;
- macro-generated unsafe code must stay inside `plugin-sdk` or an SDK-owned
  service crate;
- macros must not bypass manifest declarations or capability checks;
- macro names should describe SDK concepts, not implementation tricks.
- macros must not become the only supported API; manual trait implementations
  stay the debuggable fallback.
- macros may generate manifest snippets or validation metadata later, but the
  checked-in `plugin.toml` remains the source of static intent until the
  manifest generation story is explicit.

## Typed Feature Traits

Typed traits make common SDK surfaces predictable.

### Lua Modules

```rust
pub trait LuaModuleFeature {
    fn module_name(&self) -> &'static str;
    fn register(&self, lua: LuaModuleRegistrar<'_>) -> PluginResult<()>;
}
```

The registrar should enforce:

- the plugin manifest declares `lua.module`;
- the module name is listed in `[lua].modules`;
- the module name uses the SDK normalized module format;
- duplicate module registration fails clearly.

### Config

```rust
pub trait ConfigFeature {
    fn schema_name(&self) -> &'static str;
    fn schema_toml(&self) -> &'static str;
}
```

The registrar should register schemas through `host.configs()` and return
normal `PluginError` values on duplicate or invalid schemas.

### Files

```rust
pub trait FileProviderFeature {
    fn provider(&self) -> VirtualFileProvider<'_>;
}
```

Most plugins should implement safe open/read/close logic behind SDK-owned
adapters instead of writing raw provider callbacks themselves. Raw callbacks
remain available for low-level service code only.

### RDB

```rust
pub trait RdbPatchFeature {
    fn patch_read(&self, request: RdbPatchRead<'_>) -> PluginResult<RdbPatchResult>;
}

pub trait RdbVirtualFeature {
    fn open(&self, path: &str) -> PluginResult<Option<Box<dyn RdbVirtualFile>>>;
}

pub trait RdbVirtualFile {
    fn read_at(&mut self, offset: u64, out: &mut [u8]) -> PluginResult<usize>;
    fn size(&self) -> PluginResult<u64>;
    fn file_time(&self) -> Option<u64> {
        None
    }
}
```

The SDK implementation may keep lower-level callback forms internally, but the
public authoring API should be object-safe and pointer-free where practical.

### LinkData

```rust
pub trait LinkDataPatchFeature {
    fn replace_entry(&self, patch: LinkDataEntryPatch<'_>) -> PluginResult<PatchDecision>;
    fn patch_row(&self, patch: LinkDataRowPatch<'_>) -> PluginResult<PatchDecision>;
}
```

LinkData features should use typed entry ids, row ids, and operations instead
of ad-hoc byte patches whenever the format is known.

### Signals

```rust
pub trait SignalFeature {
    fn subscriptions(&self) -> &'static [&'static str];
    fn on_signal(&self, signal: &str, payload: &[u8]) -> PluginResult<()>;
}
```

Signals are the preferred bridge between SDK services when a direct API would
create a hard dependency.

## `sdk.rdb.patcher` Reference Migration

`sdk.rdb.patcher` is the compatibility anchor for this reset.

Current desired ownership:

- `sdk.rdb` remains the native `rdb.dll` service plugin.
- `sdk.rdb` owns the single loader-facing RDB virtual provider.
- `sdk.rdb` owns dispatch across registered RDB virtual and patch providers.
- `sdk.rdb.patcher` is a feature crate initialized by `sdk.rdb`.
- Lua module `sdk.rdb.patcher` remains available through the `sdk_rdb`
  manifest.
- `rdb.skin` behavior remains provided by `sdk_rdb`.

Target authoring shape:

```rust
impl Plugin for SdkRdb {
    const ID: &'static str = "sdk_rdb";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context
            .registrar()
            .add(RdbServiceFeature::default())?
            .add(sdk_rdb_patcher::feature())?
            .finish()
    }
}
```

`sdk_rdb_patcher::feature()` should install all patcher behavior:

- logging setup;
- Lua module registration for `sdk.rdb.patcher`;
- mod discovery integration;
- virtual RDB patch provider registration;
- character handle extensions currently exposed by the patcher.

The caller should not need to know which lower-level registrations the patcher
needs.

## Bridge Runtime Direction

The Rust feature model should be designed so it can back a future
`sdk_bridge`.

`sdk_bridge` is a native SDK plugin runtime. Its job is to adapt external
languages to SDK features while keeping SDK safety rules intact.

Responsibilities:

- load one bridge runtime implementation, such as `sdk_python`;
- own native callback trampolines;
- pin external-language objects for as long as callbacks can reach them;
- translate host return codes into runtime exceptions and runtime exceptions
  back into SDK errors;
- register Lua modules, RDB providers, LinkData providers, config schemas, and
  signal handlers through the same registrar model as Rust plugins;
- expose SDK logs and diagnostics with plugin/runtime ids.

Non-responsibilities:

- bypassing manifest capability checks;
- letting external runtimes call loader primitives directly;
- giving Python or any other runtime raw process memory access without the same
  capability gates as Rust plugins.

Possible future Python authoring shape:

```python
from oppw4_sdk import Plugin, lua_module, rdb_patch

class MyPlugin(Plugin):
    id = "my_python_plugin"
    name = "My Python Plugin"

    @lua_module("my_python_plugin")
    def module(self, lua):
        lua.set("hello", lambda: "hello from python")

    @rdb_patch()
    def patch_read(self, path, offset, data):
        return data
```

The Python author should not see `Oppw4PluginApi`, `extern "system"`, raw
pointers, or Windows calling conventions. `sdk_python` and `sdk_bridge` own
that work.

The macro layer is also the template for bridge APIs. A Python decorator should
feel like the Python equivalent of a Rust macro:

```python
@sdk_plugin(id="my_python_plugin", name="My Python Plugin")
class MyPlugin:
    @lua_module("my_python_plugin")
    def lua(self, lua):
        ...
```

Rust macros and Python decorators should describe the same SDK concepts:
simple plugin id, display name, features, capabilities, modules, providers, and
signals. The bridge can attach registry/source metadata separately.

## Implementation Phases

1. Document this API direction in `docs/PLUGIN-API.md`.
2. Add `PluginRegistrar` and feature trait skeletons to `crates/sdk-api`.
3. Add `macro_rules!` prototypes that expand to the trait layer.
4. Add simple plugin identity parsing and duplicate-id rejection rules.
5. Move low-level FFI callback adapters behind SDK-owned registrar helpers.
6. Convert `sdk.rdb` and `sdk.rdb.patcher` first.
7. Convert `sdk.linkdata`, then `sdk.runtime`, `sdk.debug`, and `sdk.overlay`.
8. Remove standalone legacy plugin shapes that are no longer useful.
9. Re-evaluate whether proc-macros are worth adding.
10. Design `sdk_bridge` only after the Rust feature layer is stable.
11. Build `sdk_python` as the first bridge runtime proof.

## Testing Requirements

The implementation should add tests at each layer:

- `PluginRegistrar` unit tests for capability checks and host-call error
  conversion;
- fake `Oppw4PluginApi` tests for Lua, config, file, RDB, LinkData, and signal
  registration;
- macro expansion smoke tests for `sdk_plugin!`, `lua_module!`,
  `config_schema!`, and RDB helper macros;
- identity tests proving duplicate active plugin ids are rejected while duplicate
  display names are allowed;
- namespace tests for simple ids, `sdk.*`, bridge ids, and registry source ids;
- `sdk.rdb` tests proving provider dispatch keeps existing open/read/close,
  seek, file-time, and patch-read behavior;
- `sdk.rdb.patcher` parity tests for the current Lua module and RDB patching
  behavior;
- compile examples for a minimal Rust plugin using only high-level feature
  traits;
- future bridge smoke tests proving a non-Rust plugin can log, register a Lua
  module, and fail cleanly.

## Rules

- Keep `Oppw4PluginApi` append-only.
- Keep `version` and `struct_size` first in ABI tables.
- Keep loader responsibilities unchanged.
- Keep unsafe FFI code isolated in SDK API/service adapters.
- Keep manifests as the source of static plugin intent.
- Keep plugin ids simple, and reject duplicate active plugin ids.
- Require capabilities before critical registration.
- Prefer typed SDK data structures over stringly-typed public APIs.
- Preserve `sdk.rdb.patcher` behavior while refactoring its shape.
- Do not preserve old public plugin forms just because they existed in the
  prototype.
