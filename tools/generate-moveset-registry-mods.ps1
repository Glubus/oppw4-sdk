param(
    [string]$ModsRoot = "D:\SteamLibrary\steamapps\common\OPPW4\mods",
    [string]$OutputRoot = "examples\js\moveset_registry_mods"
)

$ErrorActionPreference = "Stop"

$resolvedOutput = Resolve-Path -LiteralPath (Split-Path -Parent $OutputRoot) -ErrorAction SilentlyContinue
if ($null -eq $resolvedOutput) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputRoot) | Out-Null
}
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

$count = 0
Get-ChildItem -Path $ModsRoot -Directory |
    Where-Object { $_.Name -like "*_moveset" } |
    Sort-Object Name |
    ForEach-Object {
        $id = $_.Name
        $luaPath = Join-Path $_.FullName "mod.lua"
        $tomlPath = Join-Path $_.FullName "mod.toml"
        $lua = Get-Content -LiteralPath $luaPath -Raw
        $toml = Get-Content -LiteralPath $tomlPath -Raw

        $character = [regex]::Match($lua, 'character\.find\("([^"]+)"\)')
        $payload = [regex]::Match($lua, 'payload_file\s*=\s*"([^"]+)"')
        if (-not $character.Success -or -not $payload.Success) {
            throw "Cannot parse moveset mod $id"
        }

        $name = [regex]::Match($toml, '(?m)^name\s*=\s*"([^"]+)"')
        $creator = [regex]::Match($toml, '(?m)^creator\s*=\s*"([^"]+)"')
        $nameValue = if ($name.Success) { $name.Groups[1].Value } else { $id }
        $creatorValue = if ($creator.Success) { $creator.Groups[1].Value } else { "unknown" }
        $characterId = $character.Groups[1].Value
        $payloadFile = $payload.Groups[1].Value

        $modOut = Join-Path $OutputRoot $id
        New-Item -ItemType Directory -Force -Path $modOut | Out-Null

        $manifest = @"
[mod]
id = "$id"
name = "$nameValue"
creator = "$creatorValue"

[uses]
plugins = ["sdk_data", "moveset_patcher"]

[entry]
file = "main.js"
"@
        Set-Content -LiteralPath (Join-Path $modOut "mod.toml") -Value $manifest -NoNewline -Encoding UTF8

        $script = @"
import { character } from "sdk";

const target = character.find("$characterId");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for $characterId");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("$payloadFile");

oppw4.trace("$id.replace_movesets=" + JSON.stringify(result));
"@
        Set-Content -LiteralPath (Join-Path $modOut "main.js") -Value $script -NoNewline -Encoding UTF8
        $count += 1
    }

Write-Host "Generated $count moveset registry mods in $OutputRoot"
