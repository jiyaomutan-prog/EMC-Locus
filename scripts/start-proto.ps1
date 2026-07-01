param(
    [int]$Port = 8000
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$GuiRoot = Join-Path $RepoRoot "apps\gui-shell"
$LogRoot = Join-Path $RepoRoot "logs\launchers"
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

if (Test-PortInUse -Port $Port) {
    throw "Port 127.0.0.1:$Port is already in use. Stop that service or pass -Port <free-port>."
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$stdoutLog = Join-Path $LogRoot "start-proto-$timestamp.out.log"
$stderrLog = Join-Path $LogRoot "start-proto-$timestamp.err.log"
$url = "http://127.0.0.1:$Port/"

Write-Host "Starting EMC Locus static prototype at $url"
Write-Host "Logs: $stdoutLog / $stderrLog"

Start-Process `
    -FilePath "py" `
    -ArgumentList @("-m", "http.server", "$Port", "--bind", "127.0.0.1", "--directory", $GuiRoot) `
    -WorkingDirectory $RepoRoot `
    -RedirectStandardOutput $stdoutLog `
    -RedirectStandardError $stderrLog `
    -WindowStyle Hidden

Start-Sleep -Milliseconds 700
Start-Process $url
