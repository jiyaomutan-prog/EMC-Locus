param(
    [int]$Port = 8765,
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo"
)

$ErrorActionPreference = "Stop"
$LabRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$RepoRoot = (Resolve-Path (Join-Path $LabRoot "..\..")).Path
$StartLab = Join-Path $RepoRoot "scripts\start-lab.ps1"
$StopAgent = Join-Path $RepoRoot "scripts\stop-agent.ps1"
$Playwright = Join-Path $LabRoot "node_modules\.bin\playwright.cmd"

if (-not (Test-Path $Playwright)) {
    throw "Playwright executable is missing. Run npm ci or pnpm install in apps\lab-console first."
}

$env:LAB_CONSOLE_E2E_BASE_URL = "http://127.0.0.1:$Port"

try {
    & $StartLab -Port $Port -NoBrowser -Reset -PythonCommand $PythonCommand -CargoCommand $CargoCommand
    if ($LASTEXITCODE -ne 0) {
        throw "start-lab failed before E2E."
    }
    Set-Location $LabRoot
    & $Playwright test
    if ($LASTEXITCODE -ne 0) {
        throw "Playwright E2E failed."
    }
} finally {
    & $StopAgent *> $null
}
