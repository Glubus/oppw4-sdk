# Clone Optimization Notes

This document lists clone/allocation cleanup candidates found before touching implementation code.
The goal is to reduce avoidable cloning across the SDK without introducing `Arc<Mutex<_>>` as a blanket workaround.

## Rules

- Do not replace clone pressure with shared mutable state.
- Limit new `Arc<Mutex<_>>` use. Treat it as a last resort for true shared ownership across callbacks/threads, not as a default way to satisfy lifetimes.
- When state is single-owner but accessed asynchronously, prefer explicit commands/messages over shared mutable state.
- Prefer borrowing, moving ownership once, indexing, or changing return types to references where the API boundary allows it.
- Keep ABI/FFI owned strings and buffers owned. Those clones are usually required because the caller boundary is unsafe or external.
- Keep test fixture clones low priority unless they hide a production API issue.
- Optimize hot paths first: dispatch, JS module loading, analyzer loops, runtime event snapshots.

## High Priority

### JS bridge module descriptor conversion clones modules on load

Files:

- `bridges/core/src/registry/load.rs`
- `bridges/js/src/bridge.rs`

Current shape:

- `BridgeRegistry::modules_for` returns `Vec<RegistryModuleDescriptor>` by cloning registry descriptors.
- `JsBridge::load_mod` converts descriptors back into `Vec<JsModule>`.

Why it matters:

- Mod loading is less hot than dispatch, but this repeats for each JS mod.
- Descriptors contain schema and callback fields. Schema clones can be non-trivial.

Preferred direction:

- Let `BridgeModContext` borrow registry modules during load where possible.
- Or add a lightweight `JsModuleRef<'a>` for VM install/load paths so schemas and invoke callbacks are borrowed until JS context setup is complete.

Do not:

- Do not wrap the whole module list in shared mutable state.
- Do not expose borrowed module references beyond the VM lifetime unless the lifetime is explicit and enforced.

Expected impact:

- Less allocation while loading JS mods.
- Faster registry function invocation from JS.

## Medium Priority

### JS analyzer clones character and costume strings per effect

File:

- `bridges/js-analyzer/src/analyzer.rs`

Current shape:

- `character_from_receiver` returns `Option<String>`.
- `collect_costume_patch_effects` clones `character` and `costume` for model and every texture effect.

Why it matters:

- Analyzer can run often in watch/check mode.
- Big JS mods with many costume effects will allocate repeatedly.

Preferred direction:

- Keep borrowed values inside the analyzer until the final `BridgeModEffect` construction.
- Split internal parsed effect representation from exported `BridgeModEffect`:
  - internal: borrowed `&str` where source-backed
  - final report: owned strings only at the boundary

Do not:

- Do not make `BridgeModEffect` borrow strings unless the bridge core API is intentionally made lifetime-based. That would ripple through reports and JSON output.

Expected impact:

- Lower analyzer allocation count, especially for repeated texture effects.

### Bridge load inserts clone mod IDs during registration

File:

- `bridges/core/src/registry/load.rs`

Current shape:

- `load_mod` clones `mod_id` and `bridge_id` before calling bridge load.
- `register_loaded_mod` clones `mod_id` to insert effects and mod records.

Why it matters:

- Not a hot loop, but it is central API code.
- Cleanup here improves ownership clarity.

Preferred direction:

- Split validation from storage:
  - validate handler IDs using borrowed `mod_id`/`bridge_id`
  - move `mod_id` exactly once into `ModRecord`
  - store effects under a cloned key only if the map key genuinely needs to outlive the record
- Consider making `ModRecord` keyed by map only and removing duplicate `mod_id` field if redundant.

Do not:

- Do not obscure ownership by sharing IDs through reference-counted state.

Expected impact:

- Small performance gain, better API shape.

## Low Priority

### Analyzer/report JSON output clones warnings

File:

- `apps/sdk-analyzer/src/report.rs`

Current shape:

- Warning diagnostics copy file path, warning code/message, and source line into owned report diagnostics.

Why it matters:

- This is mostly output-bound.
- Owned diagnostics are convenient for JSON/human output.

Preferred direction:

- Leave as-is unless profiling shows large reports dominate.
- If needed, introduce borrowed diagnostics during formatting and only own values in JSON serialization.

Expected impact:

- Low unless huge analyzer outputs become common.

### Source and manifest tests clone temp roots

Files:

- `apps/sdk-analyzer/src/sources.rs`
- `apps/sdk-analyzer/src/manifest.rs`

Current shape:

- Tests pass `&[root.clone()]`.

Why it matters:

- Test-only allocations.

Preferred direction:

- Ignore for now.
- Optionally switch helpers to accept `&[&Path]` later if production call sites also benefit.

Expected impact:

- Negligible.

### Runtime probe snapshots clone arrays for debug/export

Files:

- `sdk/plugins/runtime/src/mission/rank/threshold_probe/snapshot.rs`
- `sdk/plugins/runtime/src/reverse/entity_counter_probe/snapshot.rs`

Current shape:

- Fixed-size/raw words are copied into output structs.
- Previous snapshots are cloned for diff/current calculations.

Why it matters:

- Some probes may run repeatedly, but many clones are for stable report ownership.

Preferred direction:

- Check call frequency before changing.
- Prefer fixed arrays in output structs where sizes are known.
- Use diff functions over references instead of cloning previous/current when possible.

Expected impact:

- Medium only if probes run every frame.

## Shared State And Message Passing

Scan result:

- No direct `std::sync::mpsc`, `Sender`, `Receiver`, `channel`, `flume`, or `crossbeam-channel` usage was found in source code.
- `crossbeam-utils` appears only in `Cargo.lock`, likely as an indirect dependency.
- Current synchronization is mostly `OnceLock<Mutex<_>>`, a few `Arc<Mutex<_>>`, one runtime `RwLock`, and `Arc<str>` for event payloads.

### Current `Arc<Mutex<_>>` entries

Files:

- `bridges/js/src/vm/handlers.rs`
- `bridges/js/src/vm/mod.rs`
- `bridges/js/src/vm/modules/mod.rs`
- `bridges/js/src/vm/modules/trace.rs`
- `sdk/plugins/runtime/src/runtime/fx/mods/state.rs`

Current shape:

- JS handler registration uses `Arc<Mutex<PendingHandlerState>>` so a QuickJS callback can push handlers while the Rust loader later drains them.
- JS logs use `Arc<Mutex<Vec<String>>>` so the exposed JS `trace` function can append logs and Rust can drain them after load/dispatch.
- FX runtime config uses `Arc<Mutex<FxRuntimeState>>` across worker/bootstrap/reload code.

Preferred direction:

- For JS handler registration, consider an owned callback state object if `rquickjs` allows non-threaded mutable callback captures safely. This is probably worth testing because handler registration is synchronous during module evaluation.
- For JS logs, a message sink abstraction is cleaner than sharing a raw `Vec<String>`:
  - short-term: `Rc<RefCell<Vec<String>>>` may be enough if QuickJS callbacks stay on one thread and `rquickjs` permits it.
  - bigger rewrite: expose a `LogSink` trait/object owned by the VM and drain through a narrow API.
  - channel option: send `VmMessage::Log(String)` to a VM-local receiver, then drain after dispatch.
- For FX runtime state, message passing is more attractive if reload/worker code becomes more active:
  - a single FX runtime owner receives `FxCommand::Reload`, `FxCommand::Apply`, `FxCommand::SetEnabled`, etc.
  - hooks write only small atomic/copy values or enqueue commands.

Do not:

- Do not add more `Arc<Mutex<Vec<_>>>` for new queues.
- Do not hold a mutex while calling plugin callbacks, host APIs, file IO, or JS functions.
- Do not switch to channels just to avoid a small lock if everything is synchronous and same-thread.

Expected impact:

- JS logs/handlers: mostly architecture cleanliness, modest runtime impact.
- FX state: potentially high correctness value if config reload and hook updates become more concurrent.

### Current `OnceLock<Mutex<_>>` globals

Files:

- `crates/host/core/src/runtime/loader/mod.rs`
- `crates/host/core/src/runtime/signals.rs`
- `crates/host/core/src/runtime/config.rs`
- `crates/host/core/src/runtime/logs.rs`
- `sdk/plugins/linkdata/src/linkdata/mod.rs`
- `sdk/plugins/rdb/patcher/src/provider.rs`
- `sdk/plugins/rdb/src/lib.rs`
- `sdk/plugins/overlay/src/panels.rs`
- `plugins/moveset_patcher/src/state.rs`
- `crates/hooks/src/winapi_file/mod.rs`
- `crates/hooks/src/signals.rs`

Current shape:

- Global registries are protected with `Mutex` because ABI callbacks and hooks need `'static` entry points.
- The bridge registry itself is behind `OnceLock<Mutex<BridgeRegistry>>`; every runtime event dispatch locks the whole registry.
- Signal subscriptions clone subscriber lists before callback dispatch, which is good because callbacks run outside the registry lock.
- Logs route through a global `LogRouter` lock and do file IO while holding that lock.

Preferred direction:

- Keep one-time immutable services as `OnceLock<T>` without a mutex when they do not mutate after init.
- For append-only registries, split registration from runtime lookup:
  - build mutable registry during initialization
  - freeze into immutable maps/slices after loading
  - dispatch reads from immutable data without locking
- For logs, prefer a dedicated logger queue:
  - producers send `LogCommand`
  - one owner thread/task performs file IO
  - callers never hold a global mutex during disk writes
- For bridge dispatch, long-term best shape is a single runtime/bridge owner:
  - external emit creates `RuntimeCommand::Dispatch(EventEnvelope)`
  - owner processes commands serially
  - no global `Mutex<BridgeRegistry>` on the hot path

Do not:

- Do not replace `OnceLock<Mutex<BridgeRegistry>>` with `Arc<Mutex<BridgeRegistry>>`; that only spreads the same bottleneck.
- Do not keep locks while invoking callbacks or JS.
- Do not create many small global locks if the data can be owned by one runtime service.

Expected impact:

- Bridge registry lock removal: high impact if events become frequent.
- Log queue: medium impact and better reentrancy safety.
- Config/schema registries: low impact; probably fine until there are many plugins.

### Message-passing candidates

There is no existing `mpsc` entry to convert today, but these places are good candidates if we accept a larger architecture change:

1. Runtime event dispatch
   - Replace direct `loader::dispatch_event` under global registry lock with `RuntimeCommand::Dispatch`.
   - One runtime owner owns `BridgeRegistry`, loaded mods, logs, and dispatch metrics.
   - This is the best "destroy and rebuild cleanly" target.

2. Logging
   - Replace `ROUTER: OnceLock<Mutex<LogRouter>>` with a log worker and `LogCommand`.
   - Useful because current code performs routing and file IO under the mutex.

3. FX director runtime
   - Replace `SharedFxState = Arc<Mutex<FxRuntimeState>>` with command messages to a single FX runtime owner.
   - Hooks should avoid allocating or blocking where possible, so only non-hook worker paths should enqueue richer commands.

4. JS VM logs
   - Replace `Arc<Mutex<Vec<String>>>` with `VmMessage::Log`.
   - Worth it only if QuickJS callback integration stays simple; otherwise a small same-thread interior-mutable sink may be cheaper.

5. RDB/linkdata providers
   - Current ABI callbacks require global access.
   - A command queue can serialize writes/patch requests, but read callbacks likely still need synchronous access to resolved virtual state.
   - Do this only if we redesign provider ownership around a runtime service.

### If breaking changes are allowed

If we are willing to break internal APIs heavily, the cleanest direction is:

- Introduce a `RuntimeEngine` owner object.
- Move `BridgeRegistry`, loaded plugin records, mod records, signal routing, and log routing under that owner.
- Expose thin ABI functions that validate pointers and send commands or call into the owner.
- Freeze registries after init where possible.
- Make hot dispatch operate on borrowed handlers and immutable lookup tables.
- Keep mutation explicit through command enums instead of shared mutable maps.

This is more work than clone cleanup, but it would remove the reason many current clones and locks exist.

## Algorithmic Complexity Hotspots

This section tracks places where the algorithm itself can become too expensive as mods, plugins, handlers, assets, or scanned memory ranges grow.

### JS module resolver sorts builtin module names but loader also stores a map

File:

- `bridges/js/src/vm/loader.rs`

Current shape:

- Builtin modules are stored as `HashMap<String, String>` in `ModLoader`.
- `ModResolver` separately stores sorted `Vec<String>` and does binary search.

Complexity:

- Resolver lookup is O(log n), already fine.
- But it duplicates the builtin module key set and does extra allocation during load.

Preferred direction:

- If resolver can own the same map shape, use a `HashSet<String>` or share an immutable key set built once.
- If module count stays tiny, leave it alone; this is not a first target.

Expected impact:

- Low to medium. Mostly cleanup unless builtin module count grows.

### Reverse probes scan memory ranges linearly

Files:

- `sdk/plugins/runtime/src/reverse/value_scan_probe/scan.rs`
- `sdk/plugins/runtime/src/reverse/entity_counter_probe/snapshot.rs`

Current shape:

- Value scan walks u16 and u32 offsets across the whole byte range.
- Entity counter diff walks u32 then u16 and checks existing u32 changes with `changes.iter().any(...)`.

Complexity:

- Value scan is O(bytes), expected for a scanner.
- Entity counter diff is O(bytes + u16_offsets * changes), bounded by `max_changes` but still worth noting.

Preferred direction:

- Keep value scan as-is unless ranges become huge; it already uses `HashSet` targets and max hit limits.
- For entity counter diff, track u32 offsets in a small set if `max_changes` grows.
- Do not optimize probe code before confirming it runs frequently enough to matter.

Expected impact:

- Low to medium. Probe scan work is often intentional.

## Probably Required Clones

These are not good first targets:

- FFI string conversion with `to_string_lossy().into_owned()`.
- Error message construction with `to_string()`.
- Serialization/report structs that must own data for JSON output.
- `PathBuf`/`String` ownership crossing plugin/host or DLL boundaries.
- Test fixture literals and expected values.

## Suggested Implementation Order

1. Audit JS `Arc<Mutex<_>>` logs/handler registration and replace with a narrower same-thread sink or messages if `rquickjs` supports it cleanly.
2. Optimize JS analyzer internals with delayed ownership for character/costume effects.
3. Revisit JS module load descriptors with borrowed module refs.
4. Consider a log worker queue if file IO under `LogRouter` lock shows up in runtime traces.
5. Only then inspect lower-priority report/probe clones if profiling still points there.

## Validation Plan

Run after each implementation step:

- `cargo fmt`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo test -p oppw4-sdk-analyzer -p sdk-js-analyzer -p sdk-js-bridge -p sdk-bridge`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo check --workspace`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo run -q -p oppw4-sdk-analyzer -- check examples/js/player_event`

Known caveat:

- Runtime plugin tests may still fail on Linux if they link Windows-only libraries such as `user32`; that is unrelated to clone cleanup.
