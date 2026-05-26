param(
    [string]$BankRoot = $PSScriptRoot,
    [string]$CrateDataPath = (Join-Path $PSScriptRoot "..\..\official_plugins\sdk\data\api\data\characters.json")
)

$ErrorActionPreference = "Stop"

$charactersDir = Join-Path $BankRoot "characters"
$generatedDir = Join-Path $BankRoot "generated"
$indexesDir = Join-Path $generatedDir "indexes"
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)

if (-not (Test-Path $charactersDir)) {
    throw "missing characters directory: $charactersDir"
}

New-Item -ItemType Directory -Force -Path $generatedDir, $indexesDir | Out-Null

function Write-TextFile($path, $text) {
    [System.IO.File]::WriteAllText($path, $text, $Utf8NoBom)
}

$characters = Get-ChildItem -LiteralPath $charactersDir -Filter "*.json" |
    Sort-Object Name |
    ForEach-Object {
        $character = Get-Content -Raw -LiteralPath $_.FullName | ConvertFrom-Json
        if ([string]::IsNullOrWhiteSpace($character.canonical)) {
            throw "character file is missing canonical id: $($_.FullName)"
        }
        $expectedName = "$($character.canonical).json"
        if ($_.Name -ne $expectedName) {
            throw "character filename must match canonical id: expected $expectedName, got $($_.Name)"
        }
        $character
    } |
    Sort-Object `
        @{ Expression = { if ($null -eq $_.model_id) { [int]::MaxValue } else { [int]$_.model_id } } }, `
        @{ Expression = { if ($null -eq $_.playable_id) { [int]::MaxValue } else { [int]$_.playable_id } } }, `
        canonical

if (@($characters).Count -eq 0) {
    throw "character bank is empty"
}

$generatedCharacters = Join-Path $generatedDir "characters.generated.json"
Write-TextFile $generatedCharacters (@($characters) | ConvertTo-Json -Depth 64)

if ($CrateDataPath) {
    $crateDir = Split-Path -Parent $CrateDataPath
    New-Item -ItemType Directory -Force -Path $crateDir | Out-Null
    Copy-Item -LiteralPath $generatedCharacters -Destination $CrateDataPath -Force
}

function Add-IndexValue($map, $key, $value) {
    if ($null -eq $key) {
        return
    }
    $name = [string]$key
    if ([string]::IsNullOrWhiteSpace($name)) {
        return
    }
    if (-not $map.Contains($name)) {
        $map[$name] = [System.Collections.Generic.List[string]]::new()
    }
    $map[$name].Add([string]$value)
}

$byAlias = [ordered]@{}
$byPlayable = [ordered]@{}
$byRuntime = [ordered]@{}
$byBoss = [ordered]@{}
$byModel = [ordered]@{}
$byMoveset = [ordered]@{}

foreach ($character in $characters) {
    $id = [string]$character.canonical
    Add-IndexValue $byAlias $id $id
    Add-IndexValue $byAlias $character.display_name $id
    Add-IndexValue $byAlias $character.model_stem $id

    if ($character.aliases) {
        foreach ($alias in $character.aliases) {
            Add-IndexValue $byAlias $alias $id
        }
    }

    Add-IndexValue $byPlayable $character.playable_id $id
    Add-IndexValue $byRuntime $character.runtime_id $id
    Add-IndexValue $byBoss $character.boss_runtime_id $id
    Add-IndexValue $byModel $character.model_id $id
    Add-IndexValue $byMoveset $character.moveset_linkdata_entry $id
}

$indexes = @{
    "by_alias.json" = $byAlias
    "by_playable_id.json" = $byPlayable
    "by_runtime_id.json" = $byRuntime
    "by_boss_runtime_id.json" = $byBoss
    "by_model_id.json" = $byModel
    "by_moveset_entry.json" = $byMoveset
}

foreach ($entry in $indexes.GetEnumerator()) {
    $output = [ordered]@{}
    foreach ($key in ($entry.Value.Keys | Sort-Object)) {
        $values = @($entry.Value[$key].ToArray() | Sort-Object -Unique)
        if ($values.Count -eq 1) {
            $output[$key] = [string]$values[0]
        } else {
            $output[$key] = $values
        }
    }
    Write-TextFile (Join-Path $indexesDir $entry.Key) ($output | ConvertTo-Json -Depth 64)
}

Write-Output "generated character bank entries=$(@($characters).Count)"
