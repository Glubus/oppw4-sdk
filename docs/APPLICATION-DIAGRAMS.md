# Application Diagrams

Ces diagrammes décrivent l'application telle qu'elle est organisée dans ce
workspace. Ils sont faits pour aider une session de debug future, notamment sur
les zones de lock, de virtualisation fichier/RDB et de buffers.

## Vue Globale

```mermaid
flowchart TB
    classDef process fill:#172033,color:#ffffff,stroke:#41516f,stroke-width:1px
    classDef host fill:#f3f6fb,color:#172033,stroke:#6b7a90,stroke-width:1px
    classDef plugin fill:#eef8f1,color:#17351f,stroke:#62a874,stroke-width:1px
    classDef bridge fill:#fff6df,color:#3a2a00,stroke:#c69a2d,stroke-width:1px
    classDef data fill:#f6f1ff,color:#271a3f,stroke:#8e6cc7,stroke-width:1px
    classDef risk fill:#fff0f0,color:#441818,stroke:#c46a6a,stroke-width:2px

    Game["OPPW4 process"] --> Loader["dinput8.dll<br>loader proxy"]
    Loader --> Host["plugin-host<br>orchestration SDK"]

    subgraph HostInternals["Host internals"]
        direction LR
        Abi["plugin-abi<br>FFI stable"]
        Api["Oppw4PluginApi<br>callbacks plugins"]
        Registry["BridgeRegistry<br>mods + handlers"]
        Hooks["WinAPI hooks<br>file virtualization"]
        Services["Services<br>logs, memory, signals, config"]
    end

    subgraph NativePlugins["Native plugins"]
        direction LR
        SdkPlugins["SDK services<br>data, runtime, debug, overlay"]
        IoPlugins["I/O services<br>linkdata, rdb"]
        ExternalPlugins["External plugins<br>moveset_patcher"]
    end

    subgraph ModRuntime["Script mod runtime"]
        direction LR
        Mods["mods/<br>directories or zip files"]
        BridgeCore["bridges/core<br>runtime contract"]
        JsBridge["bridges/js<br>QuickJS"]
        JsVm["JsVm per loaded mod"]
    end

    subgraph DataBox["Data and game files"]
        direction LR
        Oppw4Data["oppw4-data<br>schemas + generated JSON"]
        RdbData["crates/rdb + resources<br>RDB parser + hash catalog"]
        GameFiles["Game files<br>RDB / LinkData"]
    end

    Host --> HostInternals
    Host --> NativePlugins
    Host --> ModRuntime
    Host --> DataBox

    Api -.-> NativePlugins
    Registry -.-> ModRuntime
    Hooks -.-> IoPlugins
    Hooks -.-> GameFiles
    SdkPlugins -.-> Oppw4Data
    IoPlugins -.-> RdbData
    IoPlugins -.-> GameFiles

    class Game,Loader process
    class Host,Abi,Api,Registry,Services host
    class SdkPlugins,ExternalPlugins plugin
    class BridgeCore,JsBridge,Mods,JsVm bridge
    class Oppw4Data,RdbData data
    class Hooks,IoPlugins,GameFiles risk
```

## Execution Globale

```mermaid
flowchart LR
    classDef step fill:#f3f6fb,color:#172033,stroke:#6b7a90,stroke-width:1px
    classDef plugin fill:#eef8f1,color:#17351f,stroke:#62a874,stroke-width:1px
    classDef bridge fill:#fff6df,color:#3a2a00,stroke:#c69a2d,stroke-width:1px
    classDef io fill:#fff0f0,color:#441818,stroke:#c46a6a,stroke-width:2px

    Start["1. Game loads<br>dinput8.dll"] --> Init["2. Loader calls<br>plugin-host initialize()"]
    Init --> Prepare["3. Host prepares runtime<br>logs + locks + BridgeRegistry"]
    Prepare --> RegisterBridge["4. Host registers bridge runtimes<br>example: JS bridge"]
    RegisterBridge --> LoadPlugins["5. Host loads native plugins<br>SDK services then externals"]
    LoadPlugins --> PluginInit["6. Plugins init with Oppw4PluginApi<br>register capabilities/providers/modules"]
    PluginInit --> ScanMods["7. Host scans mods/<br>directories and zip files"]
    ScanMods --> DiscoverMods["8. sdk-mod-loader<br>discovers mods + reads mod.toml"]
    DiscoverMods --> Requests["9. Build BridgeLoadRequest<br>id, entry file, uses.plugins"]
    Requests --> ChooseBridge["10. BridgeRegistry chooses runtime<br>.js goes to JsBridge"]
    ChooseBridge --> LoadJs["11. JsBridge creates QuickJS VM<br>installs globals + registry stubs"]
    LoadJs --> Ready["12. Mod handlers are registered<br>runtime can dispatch events"]

    PluginInit -.-> RegistryModules["Registry modules<br>available to JS mods"]
    PluginInit -.-> VirtualProviders["File/RDB providers<br>used by WinAPI hooks"]
    RegistryModules -.-> LoadJs
    VirtualProviders -.-> GameReads["Later: game file reads<br>can be virtualized/patched"]

    class Start,Init,Prepare,ScanMods,DiscoverMods,Requests,Ready step
    class LoadPlugins,PluginInit plugin
    class RegisterBridge,ChooseBridge,LoadJs,RegistryModules bridge
    class VirtualProviders,GameReads io
```

## Bootstrap Runtime

```mermaid
  sequenceDiagram
      participant Game as OPPW4 process
      participant Loader as dinput8.dll / loader
      participant Host as plugin-host
      participant Bridges as BridgeRegistry
      participant Plugins as SDK/external plugins
      participant Mods as mods/
  
      Game->>Loader: Process starts and loads proxy
      Loader->>Host: set_logger / set_memory / set_file_provider_registrar
      Loader->>Host: initialize(game_root, plugin_root, session_stamp)
      Host->>Host: prepare_runtime()
      Host->>Host: create LOADED OnceLock<Mutex<Vec<LoadedPlugin>>>
      Host->>Host: create BRIDGES OnceLock<Mutex<BridgeRegistry>>
      Host->>Bridges: setup bridge runtimes, e.g. JS
      Host->>Plugins: discovery::load_plugins()
      Plugins-->>Host: register capabilities, config, providers, registry modules
      Host->>Mods: sdk_mod_loader::discover_mods(mods root)
      Mods-->>Host: BridgeLoadRequest list
      Host->>Bridges: load_supported_mod(request)
      Bridges-->>Host: lifecycle, handlers, boot mutations, logs
```

## Chargement Plugins

```mermaid
flowchart TD
    Start["load_plugins"] --> SdkManifests["Build synthetic SDK service manifests<br>from plugins/sdk/*.dll"]
    Start --> PluginDirs["Scan plugin_root directories<br>excluding sdk/"]
    PluginDirs --> Manifests["Read plugin.toml"]
    SdkManifests --> Merge["Merge manifests"]
    Manifests --> Merge
    Merge --> Dedup["Reject duplicate plugin ids"]
    Dedup --> Loop{"Pending manifests?"}

    Loop -->|yes| CheckDeps{"Dependencies loaded?"}
    CheckDeps -->|no| Defer["Defer manifest"]
    CheckDeps -->|yes| CheckCaps{"Required capabilities available?"}
    CheckCaps -->|no| Defer
    CheckCaps -->|yes| LoadDll["LoadLibrary entry DLL"]
    LoadDll --> InitSymbol["Get oppw4_plugin_init"]
    InitSymbol --> ApiState["Build PluginApiState<br>and Oppw4PluginApi"]
    ApiState --> Init["Call plugin init"]
    Init -->|ok| Loaded["remember_loaded_plugin<br>LOADED mutex"]
    Loaded --> AddCaps["Add provided capabilities"]
    AddCaps --> Loop
    Init -->|error| Loop
    Defer --> DeadlockGuard{"No progress in pass?"}
    DeadlockGuard -->|no| Loop
    DeadlockGuard -->|yes| Unresolved["Log unresolved manifests<br>missing deps/caps"]

    Loaded --> RegisterModule["Optional registry module registration"]
    RegisterModule --> BridgeLock["BRIDGES mutex"]
    BridgeLock --> RegistryModules["BridgeRegistry modules"]
```

## Chargement Mods Et Bridge JS

```mermaid
flowchart LR
    ModsRoot["mods root"] --> Discover["sdk-mod-loader discover_mods"]
    Discover --> DirMod["Directory with mod.toml"]
    Discover --> ZipMod["Zip with one mod.toml"]
    DirMod --> Parse["parse_mod_manifest"]
    ZipMod --> Parse
    Parse --> Request["BridgeLoadRequest<br>mod id, entry_file, uses_plugins"]

    Request --> Registry["BridgeRegistry"]
    Registry --> BridgeFor["Find exactly one runtime<br>matching entry file"]
    BridgeFor --> ModulesFor["Select registry modules<br>Always or requested by uses.plugins"]
    ModulesFor --> Context["BridgeModContext"]
    Context --> JsBridge["JsBridge load_mod"]
    JsBridge --> Runtime["QuickJS Runtime"]
    Runtime --> JsContext["QuickJS Context"]
    JsContext --> Globals["install_mod_globals"]
    Globals --> Handlers["install handler registry"]
    Handlers --> Modules["install registry modules"]
    Modules --> Api["install oppw4 API bootstrap"]
    Api --> Sandbox["hide unsafe globals"]
    Sandbox --> Entry["Evaluate JS entry module"]
    Entry --> Report["BridgeLoadReport<br>handlers + logs"]
    Report --> HandlerMap["handlers_by_event"]
```

## Registry Modules Et Appels JS

```mermaid
sequenceDiagram
    participant Plugin as SDK plugin
    participant Host as host_register_registry_module
    participant Registry as BridgeRegistry
    participant JS as QuickJS mod
    participant Callback as plugin invoke callback

    Plugin->>Host: register_registry_module(module, schema, install, invoke)
    Host->>Host: require capability registry.module
    Host->>Host: validate module allowed by plugin manifest
    Host->>Registry: register_module(descriptor)
    JS->>JS: bootstrap builds sdk.character.find stubs from schema
    JS->>Host: __oppw4_registry_invoke(qualifiedName, argsJson)
    Host->>Callback: invoke(module_context, function, args, out buffer)
    Callback-->>Host: JSON or -46 with required length
    Host-->>JS: resultJson
    JS->>JS: JSON.parse + wrapRegistryValue
```

Point à retenir pour le debug : l'invocation registry démarre avec un buffer
host de `64 * 1024` octets et retry si le callback renvoie `-46` avec une
taille requise plus grande.

## Virtualisation Fichier Et RDB

```mermaid
flowchart TB
    GameRead["Game calls WinAPI<br>CreateFileW / ReadFile / Size / Seek / Close"] --> IATHooks["IAT hooks<br>crates/hooks winapi_file"]

    IATHooks --> OpenReal{"CreateFileW<br>GENERIC_READ?"}
    OpenReal -->|provider opens| FakeHandle["Return fake virtual handle"]
    OpenReal -->|no provider| OriginalCreate["Original CreateFileW<br>then track real handle path"]

    FakeHandle --> VirtualRead["ReadFile virtual path"]
    VirtualRead --> ProviderRead["Provider read callback"]
    ProviderRead --> SdkRdbDispatch["sdk_rdb dispatch_read"]
    SdkRdbDispatch --> RdbPatcherProvider["sdk_rdb_patcher provider"]
    RdbPatcherProvider --> VirtualManager["VirtualManager<br>protected by RUNTIME mutex"]
    VirtualManager --> HandleTable["VirtualHandleTable<br>handle map to VirtualFile"]
    HandleTable --> VirtualFileNode["VirtualFile<br>prefix + mod asset reader"]

    OriginalCreate --> RealRead["ReadFile real file"]
    RealRead --> TrackRead["tracked_read gets path + offset"]
    TrackRead --> OriginalRead["Original ReadFile fills buffer"]
    OriginalRead --> PatchRead["patch_tracked_read"]
    PatchRead --> HookProviders["winapi_file providers patch_read"]
    HookProviders --> SdkRdbPatch["sdk_rdb dispatch_patch_read"]
    SdkRdbPatch --> LegacyPatchProviders["legacy PATCH_PROVIDERS"]
    SdkRdbPatch --> VirtualPatchProviders["VIRTUAL_PROVIDERS patch_read"]
    VirtualPatchProviders --> RdbIndexPatch["patch_archive_index_external_flags"]
    RdbIndexPatch --> PatchFields["Patch RDB index external size + flag"]
```

## RDB Remplacement Skin/Asset

```mermaid
flowchart TD
    Mods["Plugin or mod assets"] --> Replacements["VirtualReplacement list"]
    Replacements --> Register["register_replacements"]
    Register --> RuntimeLock["RUNTIME<br>mutex around VirtualManager"]
    Register --> Provider["VirtualFileProvider<br>open/read/size/seek/close/patch_read"]
    Provider --> HostRdb["host.rdb register_virtual_provider"]
    HostRdb --> SdkRdb["sdk_rdb register_virtual_provider"]
    SdkRdb --> RdbProviders["VIRTUAL_PROVIDERS<br>RDB provider list"]
    SdkRdb --> FileVirtualize["host.files register_virtual_provider"]
    FileVirtualize --> WinApiProviders["FILE_PROVIDERS<br>WinAPI hook provider list"]

    Replacements --> BuildTable["build virtualization table"]
    BuildTable --> SizeFields["mod_size / original_bin_size"]
    BuildTable --> Offsets["original_bin_offset / virtual_bin_offset"]
    BuildTable --> Prefix["optional virtual_prefix"]

    RuntimeLock --> Open["open_by_path_fragment_with_replacement"]
    Open --> FileNameMatch["file name or 0xhash match"]
    FileNameMatch --> OpenVirtual["open_virtual_replacement"]
    OpenVirtual --> Reader["ReplacementSource reader"]
    OpenVirtual --> PrefixPatch["Patch prefix size fields"]
    PrefixPatch --> VirtualFileNode["VirtualFile size = prefix + payload"]
```

## Carte Des Locks Et Singletons

```mermaid
flowchart TB
    subgraph HostLocks["Host locks"]
        LoadedLock["LOADED<br>loaded plugin list mutex"]
        BridgeLock["BRIDGES<br>BridgeRegistry mutex"]
        ConfigLock["CONFIG_SCHEMAS<br>config schema registry mutex"]
        LogLock["ROUTER<br>log router mutex"]
        SignalLock["SIGNALS<br>signal subscribers mutex"]
    end

    subgraph HookLocks["File hook locks"]
        ProviderLock["FILE_PROVIDERS<br>file provider list mutex"]
        OpenFileLock["OPEN_FILES<br>open file tracker mutex"]
    end

    subgraph RdbLocks["RDB locks"]
        RdbPatchLock["PATCH_PROVIDERS<br>legacy patch provider list mutex"]
        RdbVirtualLock["VIRTUAL_PROVIDERS<br>virtual provider list mutex"]
        RdbHandleLock["OPEN_HANDLES<br>RDB open handle map mutex"]
        PatcherRuntimeLock["RUNTIME<br>VirtualManager mutex"]
        SourcesLock["SOURCES<br>replacement source map mutex"]
    end

    subgraph RuntimeLocks["Runtime plugin locks"]
        FxState["SharedFxState<br>FX runtime state mutex"]
        FxInstall["INSTALL<br>FX install state mutex"]
        PlayerSnapshot["LATEST_PLAYER_SNAPSHOT<br>player snapshot RwLock"]
        ResultHash["LAST_HASH<br>result hash mutex"]
    end

    subgraph JsLocks["JS bridge locks"]
        JsLogs["JS logs<br>log vector mutex"]
        PendingHandlers["PendingHandlers<br>handler state mutex"]
    end

    BridgeLock -->|held while loading mods| JsBridgeNode["JsBridge load and dispatch"]
    ProviderLock -->|held while trying providers| OpenVirtualNode["open_virtual_fake_handle"]
    OpenFileLock -->|track/untrack/path lookup| RealReads["real file reads"]
    RdbVirtualLock -->|held while dispatching provider callbacks| PatcherRuntimeLock
    PatcherRuntimeLock -->|open/read/seek/size/patch| VirtualManagerNode["VirtualManager"]
```

Zones à surveiller plus tard :

- `BRIDGES` est verrouillé pendant `load_supported_mod` et `dispatch_event`.
- `FILE_PROVIDERS` est verrouillé pendant l'itération des providers, y compris
  l'appel `open_path`.
- `VIRTUAL_PROVIDERS` et `PATCH_PROVIDERS` sont verrouillés pendant certains
  callbacks provider dans `sdk_rdb`.
- `RUNTIME` du patcher RDB protège le `VirtualManager`, donc les opérations
  `open/read/seek/size/patch` sont sérialisées.
- Les appels provider s'enchaînent entre hooks WinAPI, `sdk_rdb`, puis patcher
  RDB. C'est la zone la plus importante à inspecter pour un blocage lié aux I/O.

## Buffers Et Tailles Fixes À Vérifier

```mermaid
flowchart LR
    RegistryInvoke["Registry invoke"] --> RegistryBuffer["Host output buffer<br>64 KiB initial"]
    RegistryBuffer --> Retry["-46 + required_len<br>resize and retry"]

    DebugMemory["sdk_debug memory scan"] --> MaxScan["MAX_SCAN_BYTES<br>4 MiB"]

    WinRead["WinAPI ReadFile"] --> ReadLen["bytes_to_read from game"]
    ReadLen --> PatchBuffer["patch_read receives<br>actual read buffer length"]
    PatchBuffer --> RdbPatch["RDB index field patching<br>only patches bytes inside read window"]

    VirtualFileNode["VirtualFile"] --> ReportedSize["size = prefix + payload"]
    VirtualFileNode --> PrefixFields["virtual prefix fields patched<br>0x08, 0x10, 0x18, 0x2c"]
    RdbIndex["RDB index"] --> ExternalSize["external size field<br>rdb_block_offset + 0x18"]
    RdbIndex --> ExternalFlag["external flag field<br>rdb_block_offset + 0x2c"]
```

Ce diagramme ne conclut pas sur la cause du blocage à 5 Mo. Il liste seulement
les endroits où une taille déclarée, une taille lue, un offset virtuel ou un
lock peut produire un symptôme de chargement bloqué.
