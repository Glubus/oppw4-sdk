param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [string]$OutDir = "dist/oppw4-sdk",
    [string]$LoaderRoot = "",
    [switch]$NoLoader,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
if (!$LoaderRoot) {
    $LoaderRoot = Join-Path $root "../oppw4-modloader"
}
$outRoot = Join-Path $root $OutDir
$pluginsRoot = Join-Path $outRoot "plugins"
$sdkRoot = Join-Path $pluginsRoot "sdk"
$targetProfile = if ($Profile -eq "release") { "release" } else { "debug" }
$targetDir = Join-Path $root "target/$Target/$targetProfile"
$loaderTargetDir = Join-Path $LoaderRoot "target/$Target/$targetProfile"
$dataRoot = Join-Path $root "oppw4-data"

$sdkPackages = @(
    "oppw4-sdk-core-plugin",
    "oppw4-sdk-data-plugin",
    "oppw4-sdk-runtime-plugin",
    "oppw4-sdk-debug-plugin",
    "oppw4-sdk-overlay-plugin",
    "oppw4-sdk-linkdata-plugin",
    "oppw4-sdk-rdb-plugin"
)
$officialPackages = @(
    "oppw4-moveset-patcher-plugin"
)

$sdkDlls = @(
    @{ Name = "sdk"; File = "sdk.dll" },
    @{ Name = "data"; File = "data.dll" },
    @{ Name = "runtime"; File = "runtime.dll" },
    @{ Name = "debug"; File = "debug.dll" },
    @{ Name = "overlay"; File = "overlay.dll" },
    @{ Name = "linkdata"; File = "linkdata.dll" },
    @{ Name = "rdb"; File = "rdb.dll" }
)
$officialPlugins = @(
    @{ Id = "moveset_patcher"; File = "moveset_patcher.dll"; Source = "plugins/moveset_patcher" }
)

function Copy-RequiredFile($source, $destination) {
    if (!(Test-Path $source)) {
        throw "missing required file: $source"
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
    Copy-Item -Force $source $destination
}

function Copy-RequiredDirectory($source, $destination) {
    if (!(Test-Path $source -PathType Container)) {
        throw "missing required directory: $source; run: git submodule update --init --recursive"
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
    Copy-Item -Recurse -Force $source $destination
}

if (!$SkipBuild) {
    $releaseFlag = if ($Profile -eq "release") { "--release" } else { "" }
    if (!$NoLoader) {
        $loaderManifest = Join-Path $LoaderRoot "Cargo.toml"
        if (!(Test-Path $loaderManifest)) {
            throw "missing loader workspace: $LoaderRoot"
        }
        $args = @("build", "--manifest-path", $loaderManifest, "-p", "oppw4-dinput8-proxy", "--target", $Target)
        if ($releaseFlag) {
            $args += $releaseFlag
        }
        & cargo @args
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for oppw4-dinput8-proxy"
        }
    }
    foreach ($package in ($sdkPackages + $officialPackages)) {
        $args = @("build", "-p", $package, "--target", $Target)
        if ($releaseFlag) {
            $args += $releaseFlag
        }
        & cargo @args
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for $package"
        }
    }
}

if (Test-Path $outRoot) {
    Remove-Item -Recurse -Force $outRoot
}
New-Item -ItemType Directory -Force -Path $sdkRoot | Out-Null

if (!$NoLoader) {
    Copy-RequiredFile (Join-Path $loaderTargetDir "dinput8.dll") (Join-Path $outRoot "dinput8.dll")
}

foreach ($dll in $sdkDlls) {
    Copy-RequiredFile (Join-Path $targetDir $dll.File) (Join-Path $sdkRoot $dll.File)
}
Copy-RequiredFile (Join-Path $root "sdk/plugins/core/plugin.toml") (Join-Path $sdkRoot "plugin.toml")

foreach ($plugin in $officialPlugins) {
    $pluginRoot = Join-Path $pluginsRoot $plugin.Id
    New-Item -ItemType Directory -Force -Path $pluginRoot | Out-Null
    Copy-RequiredFile (Join-Path $targetDir $plugin.File) (Join-Path $pluginRoot $plugin.File)
    Copy-RequiredFile (Join-Path $root "$($plugin.Source)/plugin.toml") (Join-Path $pluginRoot "plugin.toml")
}

$packageDataRoot = Join-Path $outRoot "oppw4-data"
New-Item -ItemType Directory -Force -Path $packageDataRoot | Out-Null
Copy-RequiredFile (Join-Path $dataRoot "README.md") (Join-Path $packageDataRoot "README.md")
Copy-RequiredDirectory (Join-Path $dataRoot "characters") (Join-Path $packageDataRoot "characters")
Copy-RequiredDirectory (Join-Path $dataRoot "missions") (Join-Path $packageDataRoot "missions")
Copy-RequiredDirectory (Join-Path $dataRoot "generated") (Join-Path $packageDataRoot "generated")
Copy-RequiredDirectory (Join-Path $dataRoot "schemas") (Join-Path $packageDataRoot "schemas")
New-Item -ItemType Directory -Force -Path (Join-Path $outRoot "mods") | Out-Null
Copy-RequiredDirectory (Join-Path $root "examples/js") (Join-Path $outRoot "examples/js")
Copy-RequiredDirectory (Join-Path $root "examples/rust/log_plugin") (Join-Path $outRoot "examples/rust/log_plugin")
Copy-RequiredDirectory (Join-Path $root "examples/rust/native_mod") (Join-Path $outRoot "examples/rust/native_mod")

Write-Host "SDK package written to $outRoot"
