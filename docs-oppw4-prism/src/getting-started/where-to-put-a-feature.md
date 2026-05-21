# Where To Put A Feature

Use the lowest layer that owns the concept, but do not push game meaning into lower layers.

Loader:

- DirectInput proxy;
- game root discovery;
- load `plugins/sdk/sdk.dll`;
- memory/file primitives needed by SDK core.

SDK core:

- plugin discovery and lifecycle;
- dependency order;
- capabilities;
- Lua runtime and sandbox;
- mod discovery;
- logs/config routing;
- service registration and dispatch.

SDK service plugins:

- `sdk_runtime`: runtime probes and game status providers;
- `sdk_linkdata`: LinkData patching and virtualization;
- `sdk_rdb`: RDB read/patch/virtual providers.

Feature plugins:

- skin/model/texture replacement;
- moveset patching intent;
- FX and difficulty features;
- Lua modules and `std.character` extensions.

Data bank:

- canonical character ids;
- costumes;
- models;
- textures;
- portraits;
- voices;
- body parts;
- multiple named weapons;
- moveset metadata;
- evidence and source notes.

If a feature needs to know what a Garp costume is, it probably belongs above SDK core. If it only routes a registered callback safely, SDK core may own it.
