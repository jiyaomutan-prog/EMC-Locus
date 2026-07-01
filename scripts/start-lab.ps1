param(
    [int]$Port = 8765,
    [switch]$NoBrowser,
    [switch]$Reset,
    [switch]$SeedDemo,
    [switch]$Rebuild,
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$LabIndex = Join-Path $RepoRoot "apps\lab-console\dist\index.html"
$AgentUrl = "http://127.0.0.1:$Port"
$LabUrl = "$AgentUrl/lab/"

if ($Rebuild) {
    & (Join-Path $PSScriptRoot "build-lab.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "LAB CONSOLE rebuild failed."
    }
}

if (-not (Test-Path $LabIndex)) {
    throw "LAB CONSOLE production build is missing: $LabIndex. Run .\scripts\build-lab.ps1 or .\scripts\start-lab.ps1 -Rebuild on a machine with Node/npm."
}

$agentArgs = @{
    Port = $Port
    NoQt = $true
    PythonCommand = $PythonCommand
    CargoCommand = $CargoCommand
}
if ($Reset) {
    $agentArgs.Reset = $true
}

& (Join-Path $PSScriptRoot "start-agent-qt.ps1") @agentArgs

Wait-HttpReady -Url "$AgentUrl/api/v1/health" -TimeoutSeconds 60 | Out-Null
Wait-HttpReady -Url $LabUrl -TimeoutSeconds 60 | Out-Null

if ($SeedDemo) {
    & (Join-Path $PSScriptRoot "seed-lab-demo.ps1") -AgentUrl $AgentUrl
}

Write-Host "LAB CONSOLE ready: $LabUrl"
if (-not $NoBrowser) {
    Start-Process $LabUrl
}
