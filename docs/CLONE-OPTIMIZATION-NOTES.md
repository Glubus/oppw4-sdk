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

### Bridge dispatch clones handlers per event

Files:

- `bridges/core/src/registry/dispatch.rs`
- `bridges/js/src/bridge.rs`

Current shape:

- `BridgeRegistry::dispatch_event` clones every handler with `self.handlers_for(&event.key).to_vec()`.
- `handlers_by_bridge` then groups owned `HandlerDescriptor` values.
- `JsBridge::dispatch_many` groups again with `handlers_by_mod`, cloning handler descriptors again.

Why it matters:

- This runs on every event dispatch.
- Handler descriptors contain owned IDs/keys, so cloning scales with handler count.
- The same event can batch by bridge and then by mod, multiplying allocation work.

Preferred direction:

- Group borrowed handlers: `Vec<(&BridgeId, Vec<&HandlerDescriptor>)>` or a small grouping helper over references.
- Change bridge runtime dispatch API to accept borrowed handler slices or borrowed grouped batches.
- If trait compatibility is painful, add an internal borrowed grouping layer first and keep the public trait stable until the next API cleanup.

Do not:

- Do not put handlers behind `Arc<Mutex<_>>`.
- Do not cache mutable grouped state globally; grouping depends on current registry state and event key.

Expected impact:

- Fewer per-event allocations.
- Less clone pressure in both core registry and JS bridge.

### JS bridge module descriptor conversion clones modules on load

Files:

- `bridges/core/src/registry/load.rs`
- `bridges/js/src/bridge.rs`
- `bridges/js/src/vm/modules/invoke.rs`

Current shape:

- `BridgeRegistry::modules_for` returns `Vec<RegistryModuleDescriptor>` by cloning registry descriptors.
- `JsBridge::load_mod` converts descriptors back into `Vec<JsModule>`.
- `invoke::install` then does `modules.to_vec()` into an `Arc`.

Why it matters:

- Mod loading is less hot than dispatch, but this repeats for each JS mod.
- Descriptors contain schema and callback fields. Schema clones can be non-trivial.

Preferred direction:

- Let `BridgeModContext` borrow registry modules during load where possible.
- Or add a lightweight `JsModuleRef<'a>` for VM install/load paths so schemas and invoke callbacks are borrowed until JS context setup is complete.
- For `invoke::install`, store a compact lookup table built once:
  - key: `(namespace, import_name, function_name)`
  - value: invoke callback/reference
- Avoid scanning all module schemas on every JS registry call.

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
- `declared_methods` clones method names into a `HashSet<String>`.

Why it matters:

- Analyzer can run often in watch/check mode.
- Big JS mods with many costume effects will allocate repeatedly.

Preferred direction:

- Keep borrowed values inside the analyzer until the final `BridgeModEffect` construction.
- Split internal parsed effect representation from exported `BridgeModEffect`:
  - internal: borrowed `&str` where source-backed
  - final report: owned strings only at the boundary
- For registry methods, use `HashSet<&str>` tied to `modules` lifetime instead of `HashSet<String>`.

Do not:

- Do not make `BridgeModEffect` borrow strings unless the bridge core API is intentionally made lifetime-based. That would ripple through reports and JSON output.

Expected impact:

- Lower analyzer allocation count, especially for repeated texture effects.

### Runtime player snapshot clones previous and latest state

File:

- `sdk/plugins/runtime/src/runtime/core/player.rs`

Current shape:

- `latest_snapshot()` clones the full `PlayerSnapshot`.
- `update_snapshot` clones previous state, writes a clone of the new snapshot, then compares.

Why it matters:

- Runtime hooks can call snapshot updates frequently.
- Current snapshot is small today, but the pattern will become expensive if more player context fields are added.

Preferred direction:

- Compare under the write lock before assignment:
  - acquire write lock
  - if `*latest == snapshot`, return
  - assign `snapshot`
  - publish from the stored/latest value or from a borrowed temporary before move
- If publishing needs data after assignment, build the payload from references before moving or clone only the small changed fields.

Do not:

- Do not change the store to `Arc<Mutex<PlayerSnapshot>>`; the existing `RwLock` already expresses the state ownership.
- Do not hold the lock while emitting host events.

Expected impact:

- Removes one full snapshot clone per update and keeps event emission outside the lock.

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

### Bridge grouping and conflicts use linear grouped vectors

Files:

- `bridges/core/src/registry/dispatch.rs`
- `bridges/js/src/bridge.rs`

Current shape:

- `handlers_by_bridge` groups handlers into `Vec<(BridgeId, Vec<HandlerDescriptor>)>` and uses `iter_mut().find(...)` for every handler.
- `handlers_by_mod` repeats the same pattern inside `JsBridge`.
- `unique_handler_mods` uses `mods.iter().any(...)`.
- `effect_conflicts` groups effects with a `Vec` and scans it for every effect key.

Complexity:

- Grouping is O(n * groups), worst-case O(n²).
- Conflict detection is O(effects * unique_effects), worst-case O(n²).

Preferred direction:

- Use `BTreeMap`/`HashMap` for grouping by bridge, mod, and effect conflict key.
- If deterministic output matters, use `BTreeMap` or collect from `HashMap` then sort keys before reporting.
- Combine this with borrowed handler grouping so the optimization removes both clones and O(n²) scans.

Expected impact:

- High if many mods listen to common events.
- Medium even with fewer mods because this is on dispatch/conflict paths.

### JS registry invocation scans all modules and functions per call

File:

- `bridges/js/src/vm/modules/invoke.rs`

Current shape:

- Every JS registry call parses the qualified function name.
- It then scans every module to find matching namespace/import.
- It scans functions inside the schema to check declaration.

Complexity:

- O(module_count + function_count_per_module) per JS call.

Preferred direction:

- Build a lookup table once during `invoke::install`.
- Key candidates:
  - full string: `namespace.import.function`
  - or tuple: `(namespace, import_name, function_name)`
- Value should point to the invoke callback plus any metadata needed for validation.
- If schema validation has already happened while building the table, invocation can become one hash/map lookup plus callback call.

Expected impact:

- High for mods that call registry functions often.
- Also reduces repeated string comparisons.

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

### RDB virtual replacement lookup is linear and recomputes lowercase strings

Files:

- `sdk/plugins/rdb/patcher/src/patching/virtual/manager.rs`
- `sdk/plugins/rdb/patcher/src/patching/virtual/table.rs`

Current shape:

- `find_replacement_by_path_fragment` lowercases the input path.
- For every replacement, it lowercases `replacement.file_name` and formats `0x{hash}` during lookup.
- `build_virtualization_table_from_assets` scans `assets.iter().find(...)` for every RDB file.
- `patch_archive_index_external_flags` and `data_read_hits` scan all replacements and filter by archive name on each read.

Complexity:

- Open path lookup is O(replacement_count * string_work).
- Table build is O(rdb_files * asset_count).
- Read patching is O(replacement_count) per relevant read.

Preferred direction:

- Add precomputed normalized fields to replacement/index state:
  - lowercase file name
  - hash tag string or hash lookup map
  - archive-name buckets
- Build `HashMap<lower_file_name, ModAsset>` before `build_virtualization_table_from_assets`.
- Store replacements grouped by lowercase archive name for read-time patching.
- For range overlap reads, consider sorted intervals per archive if replacement count grows.

Expected impact:

- High if generated moveset/RDB mods create many replacements.
- Very likely more valuable than micro-optimizing string clones here.

### LinkData archive entry lookup is linear

File:

- `crates/sdk/api/src/linkdata/archive/mod.rs`

Current shape:

- `entry_payload(id)` does `self.entries.iter().find(...)`.

Complexity:

- O(entry_count) per entry lookup.

Preferred direction:

- If callers request many entries from the same archive, add an index:
  - `BTreeMap<LinkDataEntryId, usize>` if deterministic/simple.
  - `HashMap<LinkDataEntryId, usize>` if hot.
- Avoid building the index for one-off parse/rebuild paths unless profiling shows it matters.

Expected impact:

- Medium for bulk patching/extraction.
- Low for single-entry reads.

### Plugin dependency/capability resolution uses case-insensitive linear scans

Files:

- `crates/host/core/src/runtime/loader/discovery/rules.rs`
- `crates/host/core/src/runtime/loader/discovery/manifests.rs`

Current shape:

- `loaded` and `capabilities` are `HashSet<String>`, but checks iterate the whole set and use `eq_ignore_ascii_case`.
- Unresolved manifest logging repeats the same pattern.

Complexity:

- O(required * loaded) and O(required * capabilities).

Preferred direction:

- Normalize IDs/capabilities to lowercase at insertion and compare with direct `HashSet::contains`.
- Wrap normalized IDs in a small newtype if we want to prevent accidental mixed-case inserts.

Expected impact:

- Medium if plugin count grows.
- Low today, but easy and clean.

### Manifest uniqueness uses vector scans

File:

- `crates/sdk/api/src/manifest.rs`

Current shape:

- `unique_strings` and `unique_registry_modules` use `Vec` plus `iter().any(eq_ignore_ascii_case)`.

Complexity:

- O(n²) in array length.

Preferred direction:

- Use a `HashSet`/`BTreeSet` of lowercase keys while preserving original output order.
- Keep the `Vec` for ordered manifest output, use the set only for membership.

Expected impact:

- Low for normal manifests.
- Good cleanup if manifests start carrying many capabilities/modules.

### RDB address tail parsing scans every byte

File:

- `crates/rdb/src/address.rs`

Current shape:

- `parse_payload_tail` tries every byte offset, then searches for a NUL byte and UTF-8 parses the slice.

Complexity:

- Worst-case O(payload_len²) behavior due to repeated tail scans.

Preferred direction:

- Scan NUL-delimited candidate strings once.
- Or search for likely address markers (`@`, `#`, `&`) and validate bounded windows around them.
- Keep `parse_block_tail` as the preferred fast path when block metadata is available.

Expected impact:

- Medium to high if `parse_payload_tail` is used on large payloads.
- Low if it is only a fallback/tooling path.

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

1. Convert bridge dispatch grouping to borrowed handlers.
2. Remove the second handler clone pass inside `JsBridge::dispatch_many`.
3. Replace O(n²) bridge conflict/grouping helpers with map-backed grouping.
4. Build a compact JS registry invoke lookup table instead of scanning modules/functions per call.
5. Audit JS `Arc<Mutex<_>>` logs/handler registration and replace with a narrower same-thread sink or messages if `rquickjs` supports it cleanly.
6. Optimize JS analyzer internals with borrowed method names and delayed ownership for effects.
7. Clean runtime player snapshot update to avoid clone-before-write.
8. Add RDB replacement indexes if generated replacement counts are expected to grow.
9. Consider a log worker queue if file IO under `LogRouter` lock shows up in runtime traces.
10. Only then inspect lower-priority report/probe clones if profiling still points there.

## Validation Plan

Run after each implementation step:

- `cargo fmt`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo test -p oppw4-sdk-analyzer -p sdk-js-analyzer -p sdk-js-bridge -p sdk-bridge`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo check --workspace`
- `CARGO_TARGET_DIR=/tmp/oppw4-sdk-target env -u RUSTC_WRAPPER cargo run -q -p oppw4-sdk-analyzer -- check examples/js/player_event`

Known caveat:

- Runtime plugin tests may still fail on Linux if they link Windows-only libraries such as `user32`; that is unrelated to clone cleanup.
