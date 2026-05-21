# Package Layout

The release package is copied into the game folder.

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
    skin_patcher/
    fx_director/
    moveset_patcher/
  mods/
```

Build the package on Linux/WSL:

```bash
tools/package-sdk.sh
```

Build the package on Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-sdk.ps1
```

Important placement rules:

- runtime mods go under `mods/`;
- plugin configs go under `plugins/configs/<plugin_id>/`;
- SDK service DLLs live together under `plugins/sdk/`;
- official feature plugins live in their own plugin folders;
- `oppw4-data/` is mandatory runtime data and should not be compiled into SDK code.
