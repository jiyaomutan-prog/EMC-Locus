param(
    [int]$Port = 8765,
    [switch]$Reset,
    [switch]$NoQt,
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo",
    [string]$CargoTargetDirectory = "target",
    [string]$StorageRootPath = "data\local-agent",
    [string]$StateName = "agent",
    [string]$AgentExecutableOverride = "",
    [string[]]$AgentArgumentPrefixOverride = @()
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
$RelativeStorageRoot = $StorageRootPath
$RelativeMigrationsRoot = "storage\sqlite"
$StorageRoot = Resolve-LauncherPath -RepoRoot $RepoRoot -Path $RelativeStorageRoot
$ResolvedCargoTargetDirectory = Resolve-LauncherPath -RepoRoot $RepoRoot -Path $CargoTargetDirectory
$DataRoot = Resolve-LauncherPath -RepoRoot $RepoRoot -Path "data"
$AgentUrl = "http://127.0.0.1:$Port"

if ($StateName -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') {
    throw "StateName contains unsupported characters: $StateName"
}

Assert-CommandAvailable $CargoCommand
Assert-CommandAvailable $PythonCommand
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

if (Test-PortInUse -Port $Port) {
    $health = Get-AgentHealth -AgentUrl $AgentUrl
    if ($null -eq $health) {
        throw "Port 127.0.0.1:$Port is in use, but EMC Locus health is not available."
    }
    Assert-AgentStorageRoot -RepoRoot $RepoRoot -AgentUrl $AgentUrl -ExpectedStorageRoot $RelativeStorageRoot | Out-Null
    Write-Host "Using existing EMC Locus agent at $AgentUrl with expected storage."
} else {
    Write-Host "Building EMC Locus agent..."
    & $CargoCommand build -q -p emc-locus-agent --target-dir $CargoTargetDirectory
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed."
    }
    $AgentExe = Join-Path $ResolvedCargoTargetDirectory "debug\emc-locus-agent.exe"
    if (-not (Test-Path $AgentExe)) {
        $AgentExe = Join-Path $ResolvedCargoTargetDirectory "debug\emc-locus-agent"
    }
    if (-not (Test-Path $AgentExe)) {
        throw "Agent executable is missing: $AgentExe"
    }
    $ServeExe = $AgentExe
    if ($AgentExecutableOverride) {
        $ServeExe = Resolve-LauncherPath -RepoRoot $RepoRoot -Path $AgentExecutableOverride
        if (-not (Test-Path $ServeExe)) {
            throw "Serve executable override is missing: $ServeExe"
        }
    }

    Write-Host "Initializing local agent storage at $StorageRoot"
    & $AgentExe storage init --storage-root $RelativeStorageRoot --migrations-root $RelativeMigrationsRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Storage initialization failed."
    }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $stdoutLog = Join-Path $paths.LogRoot "start-agent-$timestamp.out.log"
    $stderrLog = Join-Path $paths.LogRoot "start-agent-$timestamp.err.log"
    Write-Host "Starting EMC Locus agent at $AgentUrl"
    Write-Host "Logs: $stdoutLog / $stderrLog"

    $serveArguments = @($AgentArgumentPrefixOverride) + @("serve", "--storage-root", $RelativeStorageRoot, "--migrations-root", $RelativeMigrationsRoot, "--bind", "127.0.0.1:$Port")

    $process = Start-Process `
        -FilePath $ServeExe `
        -ArgumentList $serveArguments `
        -WorkingDirectory $RepoRoot `
        -RedirectStandardOutput $stdoutLog `
        -RedirectStandardError $stderrLog `
        -WindowStyle Hidden `
        -PassThru

    Write-LauncherState `
        -RuntimeRoot $paths.RuntimeRoot `
        -Name $StateName `
        -State @{
            kind = "agent"
            process_id = $process.Id
            process_name = $process.ProcessName
            repo_root = $RepoRoot
            storage_root = $StorageRoot
            port = $Port
            url = $AgentUrl
            stdout_log = $stdoutLog
            stderr_log = $stderrLog
            match_tokens = @("serve", "--storage-root", $RelativeStorageRoot, "127.0.0.1:$Port")
        }

    try {
        Wait-HttpReady -Url "$AgentUrl/api/v1/health" -TimeoutSeconds 60 -Process $process -StderrLog $stderrLog | Out-Null
        Assert-AgentStorageRoot -RepoRoot $RepoRoot -AgentUrl $AgentUrl -ExpectedStorageRoot $RelativeStorageRoot | Out-Null
    } catch {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        Remove-LauncherState -RuntimeRoot $paths.RuntimeRoot -Name $StateName
        throw
    }
}

Write-Host "Agent ready: $AgentUrl/api/v1/health"

if ($NoQt) {
    Write-Host "NoQt requested; leaving the healthy agent running."
    return
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
$qtTimestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$qtLog = Join-Path $paths.LogRoot "start-agent-qt-$qtTimestamp.log"
Write-Host "Launching Qt console connected to $AgentUrl"
& $PythonCommand "apps\qt-console\main.py" `
    --agent-url $AgentUrl `
    --projects-db (Join-Path $RelativeStorageRoot "projects.sqlite") `
    --metrology-db (Join-Path $RelativeStorageRoot "metrology.sqlite") `
    --test-definitions-db (Join-Path $RelativeStorageRoot "test_definitions.sqlite") `
    2>&1 | Tee-Object -FilePath $qtLog

exit $LASTEXITCODE
