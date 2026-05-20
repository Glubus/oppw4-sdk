# LINKDATA Unknown Entries

This folder is reserved for generated or manually promoted unknown entry maps.

The intended flow is:

1. Scan `LINKDATA_A.BIN` into an observed JSON map.
2. Generate `entry_XXXX.rs` stubs from that JSON when useful.
3. Move and rename entries into `movesets/`, `models/`, `costumes/`, or another typed folder once their role is understood.

Do not invent section names or record sizes here. Use `Generated` or `Observed` status until a dump or reverse note proves the layout.
