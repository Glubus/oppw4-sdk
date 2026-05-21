# Plugin ABI

The plugin ABI is append-only.

`Oppw4PluginApi` starts with:

- `version`;
- `struct_size`.

The version rejects incompatible ABI contracts. `struct_size` tells the reader how many bytes are actually available in the table.

Why `struct_size` matters:

- old plugins can run against newer hosts when new fields are appended;
- newer plugins can detect older hosts before reading missing callbacks;
- errors are clearer than undefined memory reads.

Rules:

- never reorder existing ABI fields;
- append new optional fields at the end;
- treat absent future fields as unavailable;
- validate the version before using callbacks;
- validate `struct_size` before reading the full current table.
