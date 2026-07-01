param(
    [string]$AgentUrl = "http://127.0.0.1:8765"
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LogRoot = Join-Path $RepoRoot "logs\launchers"
New-Item -ItemType Directory -Force -Path $LogRoot | Out-Null

function Test-AgentHealth {
    param([string]$Url)
    try {
        Invoke-RestMethod -Uri "$Url/api/v1/health" -TimeoutSec 2 | Out-Null
        return $true
    } catch {
        return $false
    }
}

function Ensure-PySide6 {
    & py -c "import PySide6" *> $null
    if ($LASTEXITCODE -eq 0) {
        return
    }
    Write-Host "PySide6 is not installed for this Python. Installing PySide6..."
    & py -m pip install PySide6
    if ($LASTEXITCODE -ne 0) {
        throw "PySide6 installation failed."
    }
}

Set-Location $RepoRoot
Ensure-PySide6

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$logPath = Join-Path $LogRoot "start-qt-demo-$timestamp.log"

if (Test-AgentHealth -Url $AgentUrl) {
    Write-Host "Launching Qt console connected to $AgentUrl"
    & py "apps\qt-console\main.py" --agent-url $AgentUrl 2>&1 | Tee-Object -FilePath $logPath
} else {
    Write-Host "Local agent is not responding at $AgentUrl; launching Qt with static bootstrap."
    & py "apps\qt-console\main.py" --bootstrap "apps\gui-shell\bootstrap.js" 2>&1 | Tee-Object -FilePath $logPath
}

exit $LASTEXITCODE
