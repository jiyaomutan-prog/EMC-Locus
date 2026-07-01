param(
    [ValidateSet("Auto", "Static", "Agent")]
    [string]$Mode = "Auto",
    [string]$AgentUrl = "http://127.0.0.1:8765",
    [string]$ExpectedStorageRoot = "data\local-agent",
    [switch]$DryRun,
    [string]$PythonCommand = "py"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
$Bootstrap = "apps\qt-console\demo\bootstrap.json"

Assert-CommandAvailable $PythonCommand
Set-Location $RepoRoot

function Test-AgentCompatible {
    if (-not (Get-AgentHealth -AgentUrl $AgentUrl)) {
        return $false
    }
    if ($ExpectedStorageRoot.Trim()) {
        try {
            Assert-AgentStorageRoot -RepoRoot $RepoRoot -AgentUrl $AgentUrl -ExpectedStorageRoot $ExpectedStorageRoot | Out-Null
        } catch {
            return $false
        }
    }
    return $true
}

$selectedMode = $Mode
if ($Mode -eq "Static") {
    $selectedMode = "Static"
} elseif ($Mode -eq "Agent") {
    if (-not (Test-AgentCompatible)) {
        throw "Mode Agent requested, but no compatible healthy agent is available at $AgentUrl."
    }
    $selectedMode = "Agent"
} else {
    if (Test-AgentCompatible) {
        $selectedMode = "Agent"
    } else {
        $selectedMode = "Static"
    }
}

if ($selectedMode -eq "Static") {
    $qtArgs = @("apps\qt-console\main.py", "--bootstrap", $Bootstrap)
    Write-Output "Qt demo mode: Static"
} else {
    $qtArgs = @("apps\qt-console\main.py", "--agent-url", $AgentUrl)
    if ($ExpectedStorageRoot.Trim()) {
        $qtArgs += @(
            "--projects-db", (Join-Path $ExpectedStorageRoot "projects.sqlite"),
            "--metrology-db", (Join-Path $ExpectedStorageRoot "metrology.sqlite"),
            "--test-definitions-db", (Join-Path $ExpectedStorageRoot "test_definitions.sqlite")
        )
    }
    Write-Output "Qt demo mode: Agent"
}

Write-Output "Qt command: $PythonCommand $($qtArgs -join ' ')"
if ($DryRun) {
    exit 0
}

function Ensure-PySide6 {
    & $PythonCommand -c "import PySide6" *> $null
    if ($LASTEXITCODE -eq 0) {
        return
    }
    Write-Host "PySide6 is not installed for this Python. Installing PySide6..."
    & $PythonCommand -m pip install PySide6
    if ($LASTEXITCODE -ne 0) {
        throw "PySide6 installation failed."
    }
}

Ensure-PySide6

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$logPath = Join-Path $paths.LogRoot "start-qt-demo-$timestamp.log"
& $PythonCommand @qtArgs 2>&1 | Tee-Object -FilePath $logPath

exit $LASTEXITCODE
