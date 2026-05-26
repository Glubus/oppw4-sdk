# Language-Independent SDK Runtime

The SDK should not be designed as "the Lua SDK".

The long-term target is a Rust runtime core that owns hooks, game events,
contexts, and typed mutations. Lua, JavaScript/TypeScript, Rhai, or any future
language should be optional frontends installed on demand.

## Decision

Split the SDK into two layers:

```text
sdk-runtime-core
  hooks
  game events
  typed contexts
  typed mutations
  reward/rank/player/mission logic

script frontends
  sdk-lua
  sdk-js
  sdk-rhai
  future VMs
```

The core must not depend on a specific scripting VM.

VMs are adapters:

1. expose the SDK API in their language;
2. let mods register callbacks;
3. receive typed runtime contexts;
4. turn script calls into typed mutations;
5. return those mutations to the core;
6. let the core apply mutations to game memory.

## Why

The current Lua direction is useful for a quick MVP, but it couples the SDK API
to one VM. That makes future JavaScript/TypeScript support harder and forces
modder-facing concepts to leak Lua-specific implementation details.

A language-independent core gives us:

- one hook implementation;
- one event model;
- one set of typed contexts;
- one mutation pipeline;
- multiple script frontends;
- easier testing without a VM;
- clearer docs: "SDK API" first, "Lua/TS/Rhai syntax" second.

The scripting language should be a frontend choice, not the architecture.

## Runtime Flow

Hooks should dispatch SDK events, not call Lua directly.

```text
reward_commit hook
  -> sdk-runtime-core builds RewardCommitEvent
  -> core dispatches event to registered script frontends
  -> frontends run callbacks and produce RewardMutations
  -> core validates and applies mutations
  -> hook returns to game
```

For rewards:

```text
RewardCommitEvent
  rank
  mission
  difficulty
  rewards

RewardMutation
  MultiplyBerry(factor)
  AddBerry(amount)
  ForceAddSouls(Vec<Soul>)
  AddCrewPoints(amount)
  ForceAddMedals(Vec<Medal>)
```

## Modder API Shape

The public API should be event-based.

TypeScript frontend:

```ts
sdk.rewards.onCommit((ctx) => {
  if (ctx.rank.contains(["S", "S+"])) {
    ctx.rewards.berry.multiply(2)
  }
})
```

Lua frontend:

```lua
sdk.rewards.on_commit(function(ctx)
  if ctx.rank:contains({ "S", "S+" }) then
    ctx.rewards.berry:multiply(2)
  end
end)
```

Rhai frontend:

```rhai
sdk.rewards.on_commit(|ctx| {
  if ctx.rank.contains(["S", "S+"]) {
    ctx.rewards.berry.multiply(2);
  }
});
```

Same event, same context, same mutation. Only the syntax changes.

## Why Not Load-Time Conditions

This looks natural but is wrong in the current load-time model:

```lua
if rewards.rank:contains({ "S", "S+" }) then
  rewards.berry:multiply(2)
end
```

At mod load, there is no mission result rank. `rank:contains(...)` can only
build a condition object. In Lua, that object is truthy, so the `if` always
passes.

The correct model is a runtime event:

```lua
sdk.rewards.on_commit(function(ctx)
  if ctx.rank:contains({ "S", "S+" }) then
    ctx.rewards.berry:multiply(2)
  end
end)
```

The `if` runs when the hook has real runtime data.

## Core Types

Keep core Rust types explicit and independent from Lua/JS/Rhai.

```text
RewardCommitEvent
  rank: Rank
  mission: Option<MissionId>
  difficulty: Option<Difficulty>
  rewards: Rewards

Rewards
  reward_berry: Option<RewardBerry>
  reward_souls: Option<RewardSouls>
  reward_medals: Option<RewardMedals>
  reward_crew_points: Option<RewardCrewPoints>

RewardBerry
  amount: u64

RewardSouls
  souls: Vec<Soul>

Soul
  type: SoulType
  count: u32

RewardMedals
  medals: Vec<Medal>

Medal
  type: MedalType
  count: u32

RewardCrewPoints
  amount: u32
```

Script frontends may accept ergonomic tables or objects, but they must parse into
these core types before the core applies anything.

## Plugin Layout

Target ownership:

```text
official_plugins/sdk/runtime
  core runtime plugin
  hooks
  event dispatch
  typed contexts
  mutation application

official_plugins/sdk/lua
  Lua VM frontend
  Lua syntax bindings
  Lua callback registry

official_plugins/sdk/js
  JS/TS VM frontend
  TypeScript definitions
  JS callback registry

official_plugins/sdk/rhai
  Rhai frontend if wanted
```

The current Lua code can stay temporarily where it is, but the direction should
be this ownership split.

## VM Installation

VMs should be installed on demand by plugin dependency/capability.

Example Lua mod:

```toml
[dependencies]
plugins = ["sdk_runtime", "sdk_lua"]
```

Example TypeScript/JS mod:

```toml
[dependencies]
plugins = ["sdk_runtime", "sdk_js"]
```

A mod should not pay for VMs it does not use.

## JavaScript/TypeScript Direction

TypeScript is probably the best long-term modder-facing API:

- more developers know JS/TS than Lua;
- `.d.ts` gives autocomplete and typed docs;
- callbacks and objects map naturally to SDK contexts;
- tooling is strong: formatter, linter, bundler, tests;
- big mods are easier to structure.

If we add JS, prefer a small embedded VM first, likely QuickJS. Do not pull in a
full Node/Deno model unless there is a strong reason.

The TypeScript layer should compile/bundle to plain JS consumed by the embedded
VM.

## Lua Direction

Lua remains useful:

- already integrated;
- light and embeddable;
- good for small scripts;
- good MVP path for event-based callbacks.

Lua should become one frontend, not the core identity of the SDK.

## Migration Plan

1. Define core event and mutation types.
   Start with rewards:

```text
RewardCommitEvent
RewardContext
RewardMutation
Rewards
RewardBerry
RewardSouls
RewardMedals
RewardCrewPoints
```

2. Add a runtime event bus inside `sdk_runtime`.
   Hooks emit typed events. Frontends register handlers.

3. Implement the reward MVP in the core.
   Start with berry because reverse notes already identify:

- `reward_commit` receives global rank as `param4`;
- slot `6` is the visible berry subtotal/total candidate.

4. Keep current staged rule/DSL APIs as temporary compatibility.
   They can compile into the same mutations internally.

5. Convert Lua to an event frontend.
   Target API:

```lua
sdk.rewards.on_commit(function(ctx)
  if ctx.rank:contains({ "S", "S+" }) then
    ctx.rewards.berry:multiply(2)
  end
end)
```

6. Move Lua-specific runtime code out of core ownership.
   Long-term target: `sdk_lua` frontend plugin.

7. Add JS/TS frontend after the core event model is stable.
   TypeScript should expose the same concepts:

```ts
sdk.rewards.onCommit((ctx) => {
  if (ctx.rank.contains(["S", "S+"])) {
    ctx.rewards.berry.multiply(2)
  }
})
```

8. Update docs to teach SDK concepts first.
   Documentation should explain `RewardContext`, `Rank`, and `Rewards` once,
   then show Lua/TS syntax variants.

## What Not To Do

Do not make hooks call Lua directly.

Do not make `sdk_runtime` depend on a specific VM.

Do not expose reverse names to modders:

- `reward_out`
- `slots`
- `param4`
- `reward_commit_detour`
- `stage_rule`

Do not make loose string maps the core model. Parse script input into typed
Rust structs.

## Immediate MVP

The smallest useful MVP is:

```lua
sdk.rewards.on_commit(function(ctx)
  if ctx.rank:contains({ "S", "S+" }) then
    ctx.rewards.berry:multiply(2)
  end
end)
```

Internally:

```text
reward_commit hook
  -> RewardCommitEvent { rank, rewards }
  -> dispatch handlers
  -> collect MultiplyBerry(2.0)
  -> apply to berry slot
```

This proves the architecture without solving every reward type immediately.

## Final Target

The SDK runtime should be language-independent.

Lua, JS/TS, Rhai, and future VMs should be optional frontends over the same Rust
event/mutation core.
