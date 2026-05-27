# Bridges

Language bridges live outside `crates/` so the host and Rust SDK stay
language-agnostic.

- `core`: bridge contract, manifests, registry, events, mutations.
- `js`: QuickJS implementation of the bridge contract.

New languages should add a sibling directory here and depend on `sdk-bridge`
from `bridges/core`.
