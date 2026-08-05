param(
    [int]$Port = 8876,
    [string]$CargoCommand = "cargo",
    [string]$NodeCommand = "node",
    [string[]]$PlaywrightArguments = @("test")
)

$ErrorActionPreference = "Stop"
$LabRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$RepoRoot = (Resolve-Path (Join-Path $LabRoot "..\..")).Path
$Playwright = Join-Path $LabRoot "node_modules\@playwright\test\cli.js"
$TargetRoot = Join-Path $RepoRoot "target\e2e-agent-build"
$AgentExecutable = Join-Path $TargetRoot "debug\emc-locus-agent.exe"
$StorageName = "e2e-lab-" + [Guid]::NewGuid().ToString("N")
$StorageRelative = "data\$StorageName"
$StorageRoot = Join-Path $RepoRoot $StorageRelative
$DataRoot = [IO.Path]::GetFullPath((Join-Path $RepoRoot "data"))
$StdoutLog = Join-Path $env:TEMP "$StorageName.out.log"
$StderrLog = Join-Path $env:TEMP "$StorageName.err.log"
$RestartedAgentPidFile = Join-Path $env:TEMP "$StorageName.restarted.pid"
$Agent = $null

if (-not (Test-Path $Playwright)) {
    throw "Playwright executable is missing. Run npm ci in apps\lab-console first."
}

if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) {
    throw "Port $Port is already in use. Choose another isolated E2E port with -Port."
}

try {
    Push-Location $RepoRoot
    try {
        $PreviousTarget = $env:CARGO_TARGET_DIR
        $env:CARGO_TARGET_DIR = $TargetRoot
        & $CargoCommand build -p emc-locus-agent
        if ($LASTEXITCODE -ne 0) {
            throw "The isolated E2E agent build failed."
        }

        & $AgentExecutable storage init --storage-root $StorageRelative --migrations-root "storage\sqlite"
        if ($LASTEXITCODE -ne 0) {
            throw "The isolated E2E storage initialization failed."
        }
    } finally {
        $env:CARGO_TARGET_DIR = $PreviousTarget
        Pop-Location
    }

    $Agent = Start-Process -FilePath $AgentExecutable `
        -ArgumentList @(
            "serve",
            "--storage-root", $StorageRelative,
            "--migrations-root", "storage\sqlite",
            "--bind", "127.0.0.1:$Port",
            "--lab-console-dist", "apps\lab-console\dist"
        ) `
        -WorkingDirectory $RepoRoot `
        -WindowStyle Hidden `
        -RedirectStandardOutput $StdoutLog `
        -RedirectStandardError $StderrLog `
        -PassThru

    $Ready = $false
    for ($Attempt = 0; $Attempt -lt 120; $Attempt++) {
        try {
            $Response = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/api/v1/health" -TimeoutSec 1
            if ($Response.StatusCode -eq 200) {
                $Ready = $true
                break
            }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if (-not $Ready) {
        $AgentError = Get-Content $StderrLog -Raw -ErrorAction SilentlyContinue
        throw "The isolated E2E agent did not become ready. $AgentError"
    }

    $env:LAB_CONSOLE_E2E_BASE_URL = "http://127.0.0.1:$Port"
    $env:LAB_CONSOLE_E2E_AGENT_PID = [string]$Agent.Id
    $env:LAB_CONSOLE_E2E_AGENT_EXECUTABLE = $AgentExecutable
    $env:LAB_CONSOLE_E2E_STORAGE_ROOT = $StorageRoot
    $env:LAB_CONSOLE_E2E_STORAGE_RELATIVE = $StorageRelative
    $env:LAB_CONSOLE_E2E_AGENT_BIND = "127.0.0.1:$Port"
    $env:LAB_CONSOLE_E2E_RESTARTED_AGENT_PID_FILE = $RestartedAgentPidFile
    $env:LAB_CONSOLE_E2E_ALLOW_AGENT_RESTART = if (
        ($PlaywrightArguments -join " ") -match "equipment-fleet-0\.22\.spec\.ts|station-material-calibration-0\.22\.1\.spec\.ts|method-workflow-0\.22\.2\.spec\.ts"
    ) { "1" } else { "0" }
    Push-Location $LabRoot
    try {
        & $NodeCommand $Playwright @PlaywrightArguments
        if ($LASTEXITCODE -ne 0) {
            throw "Playwright E2E failed."
        }
    } finally {
        Pop-Location
    }
} finally {
    if ($Agent -and -not $Agent.HasExited) {
        Stop-Process -Id $Agent.Id -Force
        Wait-Process -Id $Agent.Id -ErrorAction SilentlyContinue
    }

    if (Test-Path -LiteralPath $RestartedAgentPidFile) {
        $RestartedAgentPid = Get-Content -LiteralPath $RestartedAgentPidFile -Raw
        if ($RestartedAgentPid -match '^\d+$') {
            Stop-Process -Id ([int]$RestartedAgentPid) -Force -ErrorAction SilentlyContinue
            Wait-Process -Id ([int]$RestartedAgentPid) -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $RestartedAgentPidFile -Force -ErrorAction SilentlyContinue
    }

    if (Test-Path -LiteralPath $StorageRoot) {
        $ResolvedStorage = (Resolve-Path -LiteralPath $StorageRoot).Path
        $ExpectedPrefix = $DataRoot + [IO.Path]::DirectorySeparatorChar
        if (-not $ResolvedStorage.StartsWith($ExpectedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to clean unexpected E2E storage path: $ResolvedStorage"
        }
        Remove-Item -LiteralPath $ResolvedStorage -Recurse -Force
    }

    Remove-Item -LiteralPath $StdoutLog, $StderrLog -Force -ErrorAction SilentlyContinue
}
