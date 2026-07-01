$ErrorActionPreference = "Stop"

function Get-LauncherRepoRoot {
    return (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Initialize-LauncherPaths {
    param([string]$RepoRoot)
    $logRoot = Join-Path $RepoRoot "logs\launchers"
    $runtimeRoot = Join-Path $logRoot "runtime"
    New-Item -ItemType Directory -Force -Path $logRoot | Out-Null
    New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
    [PSCustomObject]@{
        LogRoot = $logRoot
        RuntimeRoot = $runtimeRoot
    }
}

function Resolve-LauncherPath {
    param(
        [string]$RepoRoot,
        [string]$Path
    )
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

function Assert-CommandAvailable {
    param([string]$Command)
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "Required command '$Command' was not found in PATH."
    }
}

function Get-PythonExecutable {
    param([string]$PythonCommand = "py")
    Assert-CommandAvailable $PythonCommand
    $pythonExe = (& $PythonCommand -c "import sys; print(sys.executable)" 2>$null).Trim()
    if (-not $pythonExe) {
        throw "Could not resolve the Python executable through '$PythonCommand'."
    }
    return $pythonExe
}

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

function Get-FreeLoopbackPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), 0)
    $listener.Start()
    try {
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally {
        $listener.Stop()
    }
}

function Show-LogTail {
    param(
        [string]$Path,
        [int]$Lines = 80
    )
    if ($Path -and (Test-Path $Path)) {
        Write-Host "----- $Path -----"
        Get-Content -Path $Path -Tail $Lines
        Write-Host "----- end log -----"
    }
}

function Get-LogTailText {
    param(
        [string]$Path,
        [int]$Lines = 40
    )
    if ($Path -and (Test-Path $Path)) {
        return (Get-Content -Path $Path -Tail $Lines | Out-String).Trim()
    }
    return ""
}

function Wait-HttpReady {
    param(
        [string]$Url,
        [int]$TimeoutSeconds = 30,
        [System.Diagnostics.Process]$Process = $null,
        [string]$StderrLog = $null
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $lastError = $null
    while ((Get-Date) -lt $deadline) {
        if ($Process -and $Process.HasExited) {
            try {
                $Process.Refresh()
                $Process.WaitForExit()
            } catch {
            }
            Start-Sleep -Milliseconds 200
            $stderrText = Get-LogTailText -Path $StderrLog
            Show-LogTail -Path $StderrLog
            if ($stderrText) {
                throw "Process $($Process.Id) exited with code $($Process.ExitCode) before $Url became ready. Stderr: $stderrText"
            }
            throw "Process $($Process.Id) exited with code $($Process.ExitCode) before $Url became ready."
        }
        try {
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2
            if ($response.StatusCode -eq 200) {
                return $response
            }
            $lastError = "HTTP status $($response.StatusCode)"
        } catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 250
    }
    $stderrText = Get-LogTailText -Path $StderrLog
    Show-LogTail -Path $StderrLog
    if ($stderrText) {
        throw "Timed out waiting for HTTP 200 from $Url. Last error: $lastError. Stderr: $stderrText"
    }
    throw "Timed out waiting for HTTP 200 from $Url. Last error: $lastError"
}

function Get-AgentHealth {
    param([string]$AgentUrl)
    try {
        return Invoke-RestMethod -Uri "$AgentUrl/api/v1/health" -TimeoutSec 2
    } catch {
        return $null
    }
}

function Convert-ReportedStorageRoot {
    param(
        [string]$RepoRoot,
        [string]$ReportedStorageRoot
    )
    return Resolve-LauncherPath -RepoRoot $RepoRoot -Path $ReportedStorageRoot
}

function Assert-AgentStorageRoot {
    param(
        [string]$RepoRoot,
        [string]$AgentUrl,
        [string]$ExpectedStorageRoot
    )
    $health = Get-AgentHealth -AgentUrl $AgentUrl
    if ($null -eq $health) {
        throw "Agent is not healthy at $AgentUrl."
    }
    $expected = Resolve-LauncherPath -RepoRoot $RepoRoot -Path $ExpectedStorageRoot
    $actual = Convert-ReportedStorageRoot -RepoRoot $RepoRoot -ReportedStorageRoot $health.storage_root
    if (-not [string]::Equals($expected, $actual, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Agent at $AgentUrl uses storage '$actual', expected '$expected'."
    }
    return $health
}

function Write-LauncherState {
    param(
        [string]$RuntimeRoot,
        [string]$Name,
        [hashtable]$State
    )
    $path = Join-Path $RuntimeRoot "$Name.json"
    $State["recorded_at"] = (Get-Date).ToString("o")
    $State | ConvertTo-Json -Depth 6 | Set-Content -Path $path -Encoding UTF8
}

function Read-LauncherState {
    param(
        [string]$RuntimeRoot,
        [string]$Name
    )
    $path = Join-Path $RuntimeRoot "$Name.json"
    if (-not (Test-Path $path)) {
        return $null
    }
    try {
        return Get-Content -Raw -Path $path | ConvertFrom-Json
    } catch {
        Remove-Item -LiteralPath $path -Force
        return $null
    }
}

function Remove-LauncherState {
    param(
        [string]$RuntimeRoot,
        [string]$Name
    )
    $path = Join-Path $RuntimeRoot "$Name.json"
    if (Test-Path $path) {
        Remove-Item -LiteralPath $path -Force
    }
}

function Get-ProcessCommandLine {
    param([int]$ProcessId)
    try {
        $process = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop
        return $process.CommandLine
    } catch {
        try {
            $process = Get-WmiObject Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop
            return $process.CommandLine
        } catch {
            return $null
        }
    }
}

function Test-LauncherProcessMatchesState {
    param($State)
    $process = Get-Process -Id ([int]$State.process_id) -ErrorAction SilentlyContinue
    if ($null -eq $process) {
        return $false
    }
    $commandLine = Get-ProcessCommandLine -ProcessId ([int]$State.process_id)
    if ($commandLine) {
        foreach ($token in @($State.match_tokens)) {
            if ($commandLine -notlike "*$token*") {
                return $false
            }
        }
        return $true
    }
    if ($State.process_name) {
        return [string]::Equals($process.ProcessName, $State.process_name, [System.StringComparison]::OrdinalIgnoreCase)
    }
    return $false
}

function Stop-LauncherProcess {
    param(
        [string]$RuntimeRoot,
        [string]$Name
    )
    $state = Read-LauncherState -RuntimeRoot $RuntimeRoot -Name $Name
    if ($null -eq $state) {
        Write-Host "No recorded EMC Locus $Name process."
        return
    }
    $pidValue = [int]$state.process_id
    $process = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
    if ($null -eq $process) {
        Write-Host "Removing stale $Name PID file; process $pidValue is not running."
        Remove-LauncherState -RuntimeRoot $RuntimeRoot -Name $Name
        return
    }
    if (-not (Test-LauncherProcessMatchesState -State $state)) {
        Write-Host "Removing stale $Name PID file; PID $pidValue is now another process."
        Remove-LauncherState -RuntimeRoot $RuntimeRoot -Name $Name
        return
    }
    Write-Host "Stopping EMC Locus $Name process $pidValue."
    Stop-Process -Id $pidValue -Force
    try {
        Wait-Process -Id $pidValue -Timeout 10 -ErrorAction SilentlyContinue
    } catch {
    }
    Remove-LauncherState -RuntimeRoot $RuntimeRoot -Name $Name
}
