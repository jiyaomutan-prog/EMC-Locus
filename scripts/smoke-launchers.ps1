param(
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo",
    [switch]$SkipQtOffscreen
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
$StartProto = Join-Path $PSScriptRoot "start-proto.ps1"
$StopProto = Join-Path $PSScriptRoot "stop-proto.ps1"
$StartAgentQt = Join-Path $PSScriptRoot "start-agent-qt.ps1"
$StopAgent = Join-Path $PSScriptRoot "stop-agent.ps1"
$StartQtDemo = Join-Path $PSScriptRoot "start-qt-demo.ps1"

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Body
    )
    Write-Host "== $Name"
    & $Body
    Write-Host "OK: $Name"
}

function Expect-Failure {
    param(
        [string]$Name,
        [scriptblock]$Body,
        [string]$ExpectedText = ""
    )
    Write-Host "== $Name"
    try {
        & $Body 2>&1 | Tee-Object -Variable output | Out-Null
    } catch {
        $output = @($output) + $_.Exception.Message
        $text = ($output | Out-String)
        if ($ExpectedText -and $text -notlike "*$ExpectedText*") {
            throw "Expected failure text '$ExpectedText' was not found. Output: $text"
        }
        Write-Host "OK: $Name"
        return
    }
    throw "Expected failure did not occur: $Name"
}

function Assert-HttpStatus {
    param(
        [string]$Url,
        [int]$ExpectedStatus = 200
    )
    $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 5
    if ($response.StatusCode -ne $ExpectedStatus) {
        throw "Expected HTTP $ExpectedStatus from $Url, got $($response.StatusCode)."
    }
}

function Start-DummyPortListener {
    param([int]$Port)
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), $Port)
    $listener.Start()
    return $listener
}

Set-Location $RepoRoot
$protoPort = Get-FreeLoopbackPort
$agentPort = Get-FreeLoopbackPort
$busyPort = Get-FreeLoopbackPort
$wrongStoragePort = Get-FreeLoopbackPort
$failingAgentPort = Get-FreeLoopbackPort

try {
    Invoke-Step "prototype starts, serves HTML/CSS/JS, and stops" {
        Push-Location ([System.IO.Path]::GetTempPath())
        try {
            & $StartProto -Port $protoPort -NoBrowser -PythonCommand $PythonCommand
        } finally {
            Pop-Location
        }
        Assert-HttpStatus "http://127.0.0.1:$protoPort/"
        Assert-HttpStatus "http://127.0.0.1:$protoPort/styles.css"
        Assert-HttpStatus "http://127.0.0.1:$protoPort/bootstrap.js"
        Assert-HttpStatus "http://127.0.0.1:$protoPort/app.js"
        & $StopProto
    }

    Invoke-Step "prototype refuses occupied port" {
        & $StartProto -Port $protoPort -NoBrowser -PythonCommand $PythonCommand
        try {
            Expect-Failure "start-proto occupied port" {
                & $StartProto -Port $protoPort -NoBrowser -PythonCommand $PythonCommand
            } "already in use"
        } finally {
            & $StopProto
        }
    }

    Expect-Failure "prototype detects missing Python command" {
        & $StartProto -Port (Get-FreeLoopbackPort) -NoBrowser -PythonCommand "__missing_py_for_emc_locus__"
    } "Required command"

    Invoke-Step "agent starts, exposes health, validates storage, and stops" {
        Push-Location ([System.IO.Path]::GetTempPath())
        try {
            & $StartAgentQt -Port $agentPort -NoQt -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        } finally {
            Pop-Location
        }
        Assert-HttpStatus "http://127.0.0.1:$agentPort/api/v1/health"
        Assert-AgentStorageRoot -RepoRoot $RepoRoot -AgentUrl "http://127.0.0.1:$agentPort" -ExpectedStorageRoot "data\local-agent" | Out-Null
        & $StopAgent
    }

    Invoke-Step "agent refuses non-EMC occupied port" {
        $listener = Start-DummyPortListener -Port $busyPort
        try {
            Expect-Failure "start-agent occupied by other service" {
                & $StartAgentQt -Port $busyPort -NoQt -CargoCommand $CargoCommand -PythonCommand $PythonCommand
            } "health is not available"
        } finally {
            $listener.Stop()
        }
    }

    Invoke-Step "agent refuses existing healthy agent with wrong storage" {
        & $CargoCommand build -q -p emc-locus-agent
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed for wrong-storage smoke."
        }
        $agentExe = Join-Path $RepoRoot "target\debug\emc-locus-agent.exe"
        if (-not (Test-Path $agentExe)) {
            $agentExe = Join-Path $RepoRoot "target\debug\emc-locus-agent"
        }
        & $agentExe storage init --storage-root "data\wrong-agent-smoke" --migrations-root "storage\sqlite"
        if ($LASTEXITCODE -ne 0) {
            throw "Wrong-storage initialization failed."
        }
        $wrongStdout = Join-Path $paths.LogRoot "wrong-storage-agent.out.log"
        $wrongStderr = Join-Path $paths.LogRoot "wrong-storage-agent.err.log"
        $wrongProcess = Start-Process `
            -FilePath $agentExe `
            -ArgumentList @("serve", "--storage-root", "data\wrong-agent-smoke", "--migrations-root", "storage\sqlite", "--bind", "127.0.0.1:$wrongStoragePort") `
            -WorkingDirectory $RepoRoot `
            -RedirectStandardOutput $wrongStdout `
            -RedirectStandardError $wrongStderr `
            -WindowStyle Hidden `
            -PassThru
        try {
            Wait-HttpReady -Url "http://127.0.0.1:$wrongStoragePort/api/v1/health" -TimeoutSeconds 30 -Process $wrongProcess -StderrLog $wrongStderr | Out-Null
            Expect-Failure "start-agent wrong storage" {
                & $StartAgentQt -Port $wrongStoragePort -NoQt -CargoCommand $CargoCommand -PythonCommand $PythonCommand
            } "expected"
        } finally {
            Stop-Process -Id $wrongProcess.Id -Force -ErrorAction SilentlyContinue
        }
    }

    Expect-Failure "agent startup failure surfaces stderr" {
        $pythonExe = Get-PythonExecutable -PythonCommand $PythonCommand
        $failAgent = Join-Path $paths.RuntimeRoot "fail-agent.py"
        "import sys; sys.stderr.write('intentional launcher smoke failure\n'); sys.exit(7)" | Set-Content -Path $failAgent -Encoding UTF8
        & $StartAgentQt `
            -Port $failingAgentPort `
            -NoQt `
            -CargoCommand $CargoCommand `
            -PythonCommand $PythonCommand `
            -AgentExecutableOverride $pythonExe `
            -AgentArgumentPrefixOverride @("logs\launchers\runtime\fail-agent.py")
    } "intentional launcher smoke failure"

    Invoke-Step "Qt Static mode builds command without contacting agent" {
        $output = & $StartQtDemo -Mode Static -AgentUrl "http://127.0.0.1:1" -DryRun -PythonCommand $PythonCommand 2>&1
        if (($output | Out-String) -notlike "*Qt demo mode: Static*") {
            throw "Static mode was not selected. Output: $output"
        }
    }

    Expect-Failure "Qt Agent mode fails when agent is unavailable" {
        & $StartQtDemo -Mode Agent -AgentUrl "http://127.0.0.1:1" -DryRun -PythonCommand $PythonCommand
    } "Mode Agent requested"

    Invoke-Step "Qt Auto and Agent modes select healthy compatible agent" {
        & $StartAgentQt -Port $agentPort -NoQt -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        try {
            $autoOutput = & $StartQtDemo -Mode Auto -AgentUrl "http://127.0.0.1:$agentPort" -DryRun -PythonCommand $PythonCommand 2>&1
            if (($autoOutput | Out-String) -notlike "*Qt demo mode: Agent*") {
                throw "Auto mode did not select the healthy agent. Output: $autoOutput"
            }
            $agentOutput = & $StartQtDemo -Mode Agent -AgentUrl "http://127.0.0.1:$agentPort" -DryRun -PythonCommand $PythonCommand 2>&1
            if (($agentOutput | Out-String) -notlike "*Qt demo mode: Agent*") {
                throw "Agent mode did not build an agent command. Output: $agentOutput"
            }
        } finally {
            & $StopAgent
        }
    }

    if (-not $SkipQtOffscreen) {
        Invoke-Step "Qt offscreen smoke when PySide6 is available" {
            & $PythonCommand -c "import PySide6" *> $null
            if ($LASTEXITCODE -ne 0) {
                Write-Host "PySide6 unavailable; skipping optional offscreen smoke."
                return
            }
            $pythonExe = Get-PythonExecutable -PythonCommand $PythonCommand
            $qtStdout = Join-Path $paths.LogRoot "qt-offscreen-smoke.out.log"
            $qtStderr = Join-Path $paths.LogRoot "qt-offscreen-smoke.err.log"
            $previousQtPlatform = $env:QT_QPA_PLATFORM
            $env:QT_QPA_PLATFORM = "offscreen"
            $qtProcess = Start-Process `
                -FilePath $pythonExe `
                -ArgumentList @("apps\qt-console\main.py", "--bootstrap", "apps\gui-shell\bootstrap.js") `
                -WorkingDirectory $RepoRoot `
                -RedirectStandardOutput $qtStdout `
                -RedirectStandardError $qtStderr `
                -WindowStyle Hidden `
                -PassThru
            try {
                Start-Sleep -Seconds 5
                if ($qtProcess.HasExited -and $qtProcess.ExitCode -ne 0) {
                    Show-LogTail -Path $qtStderr
                    throw "Qt offscreen smoke exited with code $($qtProcess.ExitCode)."
                }
            } finally {
                if (-not $qtProcess.HasExited) {
                    Stop-Process -Id $qtProcess.Id -Force -ErrorAction SilentlyContinue
                }
                $env:QT_QPA_PLATFORM = $previousQtPlatform
            }
        }
    }
} finally {
    & $StopProto *> $null
    & $StopAgent *> $null
}

Write-Host "Launcher smoke tests passed for $RepoRoot"
