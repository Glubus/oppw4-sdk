# Packaging

Runtime layout:

```text
OPPW4/
  dinput8.dll
  oppw4-data/
  plugins/
    sdk/
      sdk.dll
      runtime.dll
      linkdata.dll
      rdb.dll
    configs/
      <plugin_id>/
    <plugin_id>/
      plugin.toml
      <plugin_id>.dll
  mods/
    <mod_id>/
```

Rules:

- mods go under game-level `mods/`;
- plugin configs go under `plugins/configs/<plugin_id>/`;
- plugins may be folders or packaged archives later;
- the data bank can be a submodule so the community can improve it without rebuilding SDK code;
- official SDK services live together under `plugins/sdk/`.

Zip plugin loading is a good future target. It should preserve the same logical layout and keep configs outside the zip.
