param(
    [int]$Port = 8765,
    [switch]$NoBrowser,
    [switch]$Reset,
    [switch]$SeedDemo,
    [switch]$SeedEquipmentDemo,
    [switch]$SeedMeasurementDemo,
    [switch]$Rebuild,
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo"
)

$ErrorActionPreference = "Stop"

$labArgs = @{
    Port = $Port
    NoBrowser = $NoBrowser
    PythonCommand = $PythonCommand
    CargoCommand = $CargoCommand
}
if ($Reset) {
    $labArgs.Reset = $true
}
if ($SeedDemo) {
    $labArgs.SeedDemo = $true
}
if ($SeedEquipmentDemo) {
    $labArgs.SeedEquipmentDemo = $true
}
if ($SeedMeasurementDemo) {
    $labArgs.SeedMeasurementDemo = $true
}
if ($Rebuild) {
    $labArgs.Rebuild = $true
}

& (Join-Path $PSScriptRoot "start-lab.ps1") @labArgs

& (Join-Path $PSScriptRoot "start-qt-demo.ps1") `
    -Mode Agent `
    -AgentUrl "http://127.0.0.1:$Port" `
    -ExpectedStorageRoot "data\local-agent" `
    -PythonCommand $PythonCommand

exit $LASTEXITCODE
