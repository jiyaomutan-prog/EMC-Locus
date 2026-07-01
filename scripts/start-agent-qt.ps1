param(
    [int]$Port = 8765,
    [switch]$Reset
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$DataRoot = Join-Path $RepoRoot "data"
$StorageRoot = Join-Path $DataRoot "local-agent"
$MigrationsRoot = Join-Path $RepoRoot "storage\sqlite"
$LogRoot = Join-Path $RepoRoot "logs\launchers"
$AgentUrl = "http://127.0.0.1:$Port"
New-Item -ItemType Directory -Force -Path $LogRoot | Out-Null

function Test-PortInUse {
    param([int]$Port)
    $listener = $null
    try {
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), $Port)
        $listener.Start()
        return $false
    } catch {
        return $true
    } finally {
        if ($null -ne $listener) {
            $listener.Stop()
        }
    }
}

function Test-AgentHealth {
    param([string]$Url)
    try {
        Invoke-RestMethod -Uri "$Url/api/v1/health" -TimeoutSec 2 | Out-Null
        return $true
    } catch {
        return $false
    }
}

function Wait-AgentHealth {
    param([string]$Url)
    for ($i = 0; $i -lt 60; $i++) {
        if (Test-AgentHealth -Url $Url) {
            return
        }
        Start-Sleep -Seconds 1
    }
    throw "Agent did not become healthy at $Url within 60 seconds."
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

if ($Reset) {
    if (Test-PortInUse -Port $Port) {
        throw "Cannot reset while 127.0.0.1:$Port is in use. Stop the running agent first."
    }
    if (Test-Path $StorageRoot) {
        New-Item -ItemType Directory -Force -Path $DataRoot | Out-Null
        $resolvedDataRoot = (Resolve-Path $DataRoot).Path
        $resolvedStorageRoot = (Resolve-Path $StorageRoot).Path
        if (-not $resolvedStorageRoot.StartsWith($resolvedDataRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to reset storage outside data root: $resolvedStorageRoot"
        }
        Write-Host "Resetting local agent storage at $resolvedStorageRoot"
        Remove-Item -LiteralPath $resolvedStorageRoot -Recurse -Force
    }
}

New-Item -ItemType Directory -Force -Path $StorageRoot | Out-Null
Ensure-PySide6

if (Test-PortInUse -Port $Port) {
    if (-not (Test-AgentHealth -Url $AgentUrl)) {
        throw "Port 127.0.0.1:$Port is in use, but EMC Locus health is not available."
    }
    Write-Host "Using existing EMC Locus agent at $AgentUrl"
} else {
    Write-Host "Initializing local agent storage at $StorageRoot"
    & cargo run -q -p emc-locus-agent -- storage init --storage-root $StorageRoot --migrations-root $MigrationsRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Storage initialization failed."
    }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $stdoutLog = Join-Path $LogRoot "start-agent-$timestamp.out.log"
    $stderrLog = Join-Path $LogRoot "start-agent-$timestamp.err.log"
    Write-Host "Starting EMC Locus agent at $AgentUrl"
    Write-Host "Logs: $stdoutLog / $stderrLog"
    Start-Process `
        -FilePath "cargo" `
        -ArgumentList @("run", "-q", "-p", "emc-locus-agent", "--", "serve", "--storage-root", $StorageRoot, "--migrations-root", $MigrationsRoot, "--bind", "127.0.0.1:$Port") `
        -WorkingDirectory $RepoRoot `
        -RedirectStandardOutput $stdoutLog `
        -RedirectStandardError $stderrLog `
        -WindowStyle Hidden
    Wait-AgentHealth -Url $AgentUrl
}

$qtTimestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$qtLog = Join-Path $LogRoot "start-agent-qt-$qtTimestamp.log"
Write-Host "Launching Qt console connected to $AgentUrl"
& py "apps\qt-console\main.py" `
    --agent-url $AgentUrl `
    --projects-db (Join-Path $StorageRoot "projects.sqlite") `
    --metrology-db (Join-Path $StorageRoot "metrology.sqlite") `
    --test-definitions-db (Join-Path $StorageRoot "test_definitions.sqlite") `
    2>&1 | Tee-Object -FilePath $qtLog

exit $LASTEXITCODE
