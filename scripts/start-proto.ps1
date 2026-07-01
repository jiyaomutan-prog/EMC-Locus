param(
    [int]$Port = 8000,
    [switch]$NoBrowser,
    [string]$PythonCommand = "py"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
$GuiIndex = Join-Path $RepoRoot "apps\gui-shell\index.html"
$RelativeGuiRoot = "apps\gui-shell"
$url = "http://127.0.0.1:$Port/"

if (-not (Test-Path $GuiIndex)) {
    throw "Static prototype entry point is missing: $GuiIndex"
}

$PythonExe = Get-PythonExecutable -PythonCommand $PythonCommand

if (Test-PortInUse -Port $Port) {
    throw "Port 127.0.0.1:$Port is already in use. Stop that service or pass -Port <free-port>."
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$stdoutLog = Join-Path $paths.LogRoot "start-proto-$timestamp.out.log"
$stderrLog = Join-Path $paths.LogRoot "start-proto-$timestamp.err.log"

Write-Host "Starting EMC Locus static prototype at $url"
Write-Host "Logs: $stdoutLog / $stderrLog"

$process = Start-Process `
    -FilePath $PythonExe `
    -ArgumentList @("-m", "http.server", "$Port", "--bind", "127.0.0.1", "--directory", $RelativeGuiRoot) `
    -WorkingDirectory $RepoRoot `
    -RedirectStandardOutput $stdoutLog `
    -RedirectStandardError $stderrLog `
    -WindowStyle Hidden `
    -PassThru

Write-LauncherState `
    -RuntimeRoot $paths.RuntimeRoot `
    -Name "proto" `
    -State @{
        kind = "proto"
        process_id = $process.Id
        process_name = $process.ProcessName
        repo_root = $RepoRoot
        port = $Port
        url = $url
        stdout_log = $stdoutLog
        stderr_log = $stderrLog
        match_tokens = @("http.server", "$Port", $RelativeGuiRoot)
    }

try {
    Wait-HttpReady -Url $url -TimeoutSeconds 20 -Process $process -StderrLog $stderrLog | Out-Null
} catch {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Remove-LauncherState -RuntimeRoot $paths.RuntimeRoot -Name "proto"
    throw
}

Write-Host "Prototype ready: $url"
if (-not $NoBrowser) {
    Start-Process $url
}
