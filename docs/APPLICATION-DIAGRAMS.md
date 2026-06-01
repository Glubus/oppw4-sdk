# Application Diagrams

These diagrams describe the current branch: host/runtime, `sdk-mod-loader`,
`sdkt`, `js-analyzer`, and the JS bridge.

## Overview

```mermaid
flowchart TB
    Game["OPPW4 process"] --> Loader["dinput8.dll<br>loader proxy"]
    Loader --> Host["plugin-host<br>host runtime"]

    Host --> Abi["plugin ABI<br>stable FFI"]
    Host --> Plugins["SDK plugins<br>data, runtime, debug, overlay, linkdata, rdb"]
    Host --> ModLoader["sdk-mod-loader<br>mods discovery"]
    Host --> BridgeRegistry["BridgeRegistry<br>modules, runtimes, handlers"]
    Host --> Logs["host logs<br>mods and plugins"]

    ModLoader --> Mods["mods/<br>directories or zip archives"]
    BridgeRegistry --> JsBridge["bridges/js<br>QuickJS runtime"]
    BridgeRegistry --> JsAnalyzer["bridges/js-analyzer<br>static analysis"]
    JsAnalyzer --> Sdkt["apps/sdkt<br>sdkt CLI"]

    Plugins --> Data["oppw4-data<br>schemas and generated data"]
    Plugins --> BridgeRegistry
    JsBridge --> BridgeRegistry
```

## Runtime Flow

```mermaid
flowchart LR
    Start["1. Game loads<br>dinput8.dll"] --> Init["2. Loader calls<br>plugin_host initialize"]
    Init --> Prepare["3. Host prepares runtime<br>locks and log router"]
    Prepare --> Setup["4. Register bridge runtimes<br>example: JS bridge"]
    Setup --> LoadPlugins["5. Load SDK plugins<br>from plugin_root"]
    LoadPlugins --> PluginInit["6. Plugins init through<br>Oppw4PluginApi"]
    PluginInit --> ScanMods["7. Scan mods/<br>with sdk-mod-loader"]
    ScanMods --> Parse["8. Parse mod.toml<br>directory or zip"]
    Parse --> Request["9. Build BridgeLoadRequest<br>mod id, entry file, uses.plugins"]
    Request --> Choose["10. BridgeRegistry picks runtime<br>.js goes to JsBridge"]
    Choose --> LoadJs["11. JsBridge builds QuickJS VM<br>installs globals and registry stubs"]
    LoadJs --> Ready["12. Mod is loaded<br>handlers and logs registered"]
```

## Mod Loader

```mermaid
flowchart TB
    Root["mods root"] --> Discover["discover_mods"]
    Discover --> Dir["directory mod<br>contains mod.toml"]
    Discover --> Zip["zip mod<br>contains mod.toml"]
    Dir --> ParseDir["parse_mod_manifest"]
    Zip --> ParseZip["read zip manifest<br>then parse_mod_manifest"]
    ParseDir --> Discovered["DiscoveredMod"]
    ParseZip --> Discovered
    Discovered --> Source["ModSource<br>Directory or Zip"]
    Discovered --> Manifest["ModManifest<br>id, name, runtime, uses, entry"]
```

## Analyzer Flow

```mermaid
flowchart TB
    Editor["Editor or CLI"] --> Sdkt["sdkt"]
    Sdkt --> Check["sdkt check"]
    Sdkt --> Watch["sdkt check --watch"]
    Sdkt --> Install["sdkt init/install"]

    Check --> Roots["input roots"]
    Watch --> Roots
    Roots --> ManifestDiag["manifest diagnostics"]
    Roots --> SourceFiles["source snapshot and file scan"]
    SourceFiles --> JsAnalyze["sdk-js-analyzer::analyze"]
    JsAnalyze --> Contracts["registry contracts<br>registry_module descriptors"]
    JsAnalyze --> Effects["bridge mod effects"]
    JsAnalyze --> Warnings["analysis warnings"]
    SourceFiles --> Imports["relative import validation"]
    SourceFiles --> Assets["asset validation"]
    ManifestDiag --> Diagnostics["diagnostics"]
    Imports --> Diagnostics
    Assets --> Diagnostics
    Effects --> Report["final report"]
    Warnings --> Report
    Diagnostics --> Report
```

## Bridge JS Runtime

```mermaid
flowchart TB
    Registry["BridgeRegistry"] --> LoadMod["load_supported_mod"]
    LoadMod --> Context["BridgeModContext"]
    Context --> VmLoad["vm::load"]
    VmLoad --> QuickJs["QuickJS Runtime + Context"]
    QuickJs --> Globals["install_mod_globals"]
    QuickJs --> Handlers["install handler registry"]
    QuickJs --> RegistryModules["install registry modules"]
    QuickJs --> Api["install bridge API bootstrap"]
    QuickJs --> Seal["hide unsafe globals"]
    Seal --> Entry["evaluate JS entry module"]
    Entry --> Vm["JsVm"]
    Vm --> Dispatch["dispatch event handlers"]
    Vm --> Logs["drain logs"]
```

## Lock Surface

```mermaid
flowchart TB
    HostLocks["host locks"] --> Loaded["LOADED<br>loaded plugin list"]
    HostLocks --> Bridges["BRIDGES<br>BridgeRegistry"]
    HostLocks --> LogsLock["ROUTER<br>log router"]
    HostLocks --> Signals["SIGNALS<br>signal subscribers"]

    FileLocks["file hook locks"] --> Providers["FILE_PROVIDERS<br>virtual file providers"]
    FileLocks --> OpenFiles["OPEN_FILES<br>file tracker"]

    RdbLocks["RDB locks"] --> RdbPatch["PATCH_PROVIDERS<br>patch providers"]
    RdbLocks --> RdbVirtual["VIRTUAL_PROVIDERS<br>virtual providers"]
    RdbLocks --> RdbHandles["OPEN_HANDLES<br>handle map"]
    RdbLocks --> VirtualManager["RUNTIME<br>VirtualManager"]

    Bridges --> JsVM["JsBridge and JsVm"]
    Providers --> FileIO["file reads and virtual reads"]
    VirtualManager --> RdbIO["RDB patch and replacement path"]
```

This document is intentionally high level. It is meant to make the runtime shape
and the debug surface obvious before digging into a specific bug.
