# Roadmap

Current completed direction:

- loader is kept minimal;
- SDK core owns plugin orchestration;
- SDK services own game-specific systems;
- Lua std modules are split under `std_plugins/`;
- data bank lives outside code as JSON;
- config schema registration exists in the ABI;
- plugin ABI includes `struct_size`.

Near-term work:

- fill out `oppw4-data` progressively with community validation;
- stabilize `skin_patcher` replacement APIs over costumes, assets, and body parts;
- continue `moveset_patcher` over resolved targets only;
- investigate `fx_director` with the game binary and LinkData available;
- add zip plugin loading while keeping configs under `plugins/configs/`;
- expand this book from skeleton docs into full reference pages.

Longer-term work:

- rendered architecture diagrams with `mdbook-mermaid`;
- generated API reference from Rust doc comments;
- modder examples with real packages;
- stricter data schema validation and contributor tooling.
