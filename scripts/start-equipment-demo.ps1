param(
    [int]$Port = 8765,
    [switch]$NoBrowser,
    [switch]$Reset,
    [switch]$Rebuild,
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo"
)

$ErrorActionPreference = "Stop"

$labArgs = @{
    Port = $Port
    NoBrowser = $NoBrowser
    SeedEquipmentDemo = $true
    PythonCommand = $PythonCommand
    CargoCommand = $CargoCommand
}
if ($Reset) {
    $labArgs.Reset = $true
}
if ($Rebuild) {
    $labArgs.Rebuild = $true
}

& (Join-Path $PSScriptRoot "start-lab.ps1") @labArgs
