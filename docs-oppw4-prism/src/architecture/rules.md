# Rules

Hard architecture rules:

- keep `dinput8.dll` minimal;
- do not add Lua, character bank, LinkData, RDB, skin, FX, or moveset logic to the loader;
- SDK core orchestrates and validates, but does not own feature behavior;
- game format behavior belongs in SDK service plugins;
- user-facing feature behavior belongs in feature plugins;
- Lua mods use SDK modules instead of raw filesystem/process access;
- plugin manifests must declare dependencies, Lua modules, and capabilities;
- plugin ABI structs are append-only;
- keep `version` and `struct_size` first in ABI tables;
- generated data must not be hand-edited.

File organization rules:

- one responsibility per file;
- split production files that mix multiple decisions;
- use folders for modules that are expected to grow;
- keep tests near the module they verify;
- keep unsafe code isolated from parsing and table-building code.

Documentation rules:

- repository docs are written in English;
- public APIs need examples;
- reverse-engineering notes must separate confirmed facts from guesses;
- record new architecture decisions before implementing broad changes.
