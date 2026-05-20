# Character Bank

The character bank is central SDK data.

It is intended to become the canonical source for all known OPPW4 character
metadata:

- canonical ids and aliases;
- playable/runtime/boss/model ids;
- models, forms, costumes, slots, and file stems;
- text keys and UI labels;
- portraits, icons, materials, and related assets;
- LinkData entries;
- RDB references;
- relationships between base characters, variants, forms, and DLC rows;
- evidence sources.

The editable source of truth is one JSON file per character:

```text
characters/
  luffy.json
  zoro.json
  law.json
  ...
```

Generated machine-facing views live under:

```text
generated/
  characters.generated.json
  indexes/
    by_alias.json
    by_runtime_id.json
    by_model_id.json
    by_moveset_entry.json
```

Humans edit `characters/*.json`. Runtime code and tools should consume generated
views or generated Rust data. Do not hand-edit `generated/*`.

Regenerate the unified bank and indexes with:

```powershell
powershell -ExecutionPolicy Bypass -File resources/character_bank/generate.ps1
```

The additional domain files (`models.json`, `linkdata.json`, `text.json`,
`assets.json`) are placeholders for the richer split-bank model described in
the SDK RFC.
