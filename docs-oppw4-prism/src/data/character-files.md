# Character Files

Each character has editable data under:

```text
oppw4-data/characters/<character>/data.json
```

The character file groups identity, IDs, costumes, assets, LinkData references,
RDB references, and evidence. Generated SDK views are produced from these source
files.

Keep source files human-editable:

- use stable IDs and names;
- prefer explicit arrays over magic strings;
- keep uncertain information in notes/evidence fields;
- avoid changing generated files by hand.
