param(
    [string]$PythonCommand = "py",
    [string]$CargoCommand = "cargo",
    [switch]$SkipQtOffscreen
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
$StartLab = Join-Path $PSScriptRoot "start-lab.ps1"
$StopAgent = Join-Path $PSScriptRoot "stop-agent.ps1"
$StartQtDemo = Join-Path $PSScriptRoot "start-qt-demo.ps1"
$SeedLabDemo = Join-Path $PSScriptRoot "seed-lab-demo.ps1"
$SeedEquipmentDemo = Join-Path $PSScriptRoot "seed-equipment-demo.ps1"
$SeedMeasurementDemo = Join-Path $PSScriptRoot "seed-measurement-engineering-demo.ps1"

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
    return $response
}

function Assert-RedirectToLab {
    param([string]$Url)
    $response = $null
    try {
        $request = [System.Net.HttpWebRequest]::Create($Url)
        $request.AllowAutoRedirect = $false
        $request.Timeout = 5000
        $response = $request.GetResponse()
    } catch {
        $response = $_.Exception.Response
    }
    if ($null -eq $response) {
        throw "No redirect response from $Url."
    }
    $status = [int]$response.StatusCode
    $location = $response.Headers["Location"]
    if ($status -lt 300 -or $status -gt 399 -or $location -ne "/lab/") {
        throw "Expected redirect to /lab/ from $Url, got status $status and Location '$location'."
    }
    $response.Close()
}

function Start-DummyPortListener {
    param([int]$Port)
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), $Port)
    $listener.Start()
    return $listener
}

function Assert-LabAssetServed {
    param([string]$AgentUrl)
    $index = Assert-HttpStatus "$AgentUrl/lab/"
    $matches = [regex]::Matches($index.Content, "/lab/assets/[^`"']+")
    if ($matches.Count -eq 0) {
        throw "LAB index does not reference built assets."
    }
    foreach ($match in $matches) {
        Assert-HttpStatus "$AgentUrl$($match.Value)" | Out-Null
    }
}

Set-Location $RepoRoot
$labPort = Get-FreeLoopbackPort
$busyPort = Get-FreeLoopbackPort

try {
    Invoke-Step "LAB starts from a path with spaces, serves API, root redirect, SPA, assets, and stops" {
        Push-Location ([System.IO.Path]::GetTempPath())
        try {
            & $StartLab -Port $labPort -NoBrowser -Reset -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        } finally {
            Pop-Location
        }
        $agentUrl = "http://127.0.0.1:$labPort"
        Assert-HttpStatus "$agentUrl/api/v1/health" | Out-Null
        Assert-AgentStorageRoot -RepoRoot $RepoRoot -AgentUrl $agentUrl -ExpectedStorageRoot "data\local-agent" | Out-Null
        Assert-RedirectToLab "$agentUrl/"
        Assert-HttpStatus "$agentUrl/lab/" | Out-Null
        Assert-HttpStatus "$agentUrl/lab/templates/does-not-exist" | Out-Null
        Assert-LabAssetServed -AgentUrl $agentUrl
        & $StopAgent
    }

    Invoke-Step "LAB seed uses public API and library data becomes visible through API" {
        & $StartLab -Port $labPort -NoBrowser -Reset -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        try {
            $agentUrl = "http://127.0.0.1:$labPort"
            & $SeedLabDemo -AgentUrl $agentUrl
            $templates = Invoke-RestMethod -Uri "$agentUrl/api/v1/test-templates" -TimeoutSec 10
            $ids = @($templates.test_templates | ForEach-Object { $_.identity.template_id })
            foreach ($expected in @("DEMO-APPROVED-001", "DEMO-DRAFT-001", "DEMO-RICH-001")) {
                if ($ids -notcontains $expected) {
                    throw "Seeded template missing from API library: $expected"
                }
            }
        } finally {
            & $StopAgent
        }
    }

    Invoke-Step "Equipment seed uses public API and catalog data becomes visible through API" {
        & $StartLab -Port $labPort -NoBrowser -Reset -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        try {
            $agentUrl = "http://127.0.0.1:$labPort"
            & $SeedEquipmentDemo -AgentUrl $agentUrl
            $models = Invoke-RestMethod -Uri "$agentUrl/api/v1/equipment-models" -TimeoutSec 10
            $modelIds = @($models.equipment_models | ForEach-Object { $_.identity.equipment_model_id })
            foreach ($expected in @("EQM-DEMO-NRP6AN-FWD", "EQM-DEMO-SERIAL-AMP", "EQM-DEMO-CAN-BUS-POWER", "EQM-DEMO-MANUAL-ANTENNA", "EQM-PRESET-ADC-CONVERTER", "EQM-PRESET-DAQ-CARD")) {
                if ($modelIds -notcontains $expected) {
                    throw "Seeded equipment model missing from API library: $expected"
                }
            }
            $drivers = Invoke-RestMethod -Uri "$agentUrl/api/v1/driver-profiles" -TimeoutSec 10
            $driverIds = @($drivers.driver_profiles | ForEach-Object { $_.identity.driver_profile_id })
            foreach ($expected in @("DRV-DEMO-NRP6AN-SCPI", "DRV-DEMO-SERIAL-AMP", "DRV-DEMO-CAN-BUS-POWER")) {
                if ($driverIds -notcontains $expected) {
                    throw "Seeded driver profile missing from API library: $expected"
                }
            }
        } finally {
            & $StopAgent
        }
    }

    Invoke-Step "Measurement engineering seed uses public API and definitions become visible through API" {
        & $StartLab -Port $labPort -NoBrowser -Reset -CargoCommand $CargoCommand -PythonCommand $PythonCommand
        try {
            $agentUrl = "http://127.0.0.1:$labPort"
            & $SeedMeasurementDemo -AgentUrl $agentUrl
            $curves = Invoke-RestMethod -Uri "$agentUrl/api/v1/engineering-curves" -TimeoutSec 10
            $curveIds = @($curves.items | ForEach-Object { $_.identity.entity_id })
            foreach ($expected in @("CURVE-DEMO-CURRENT-PROBE-TRANSFER", "CURVE-DEMO-BICONICAL-ANTENNA-FACTOR", "CURVE-DEMO-RF-CABLE-1M-LOSS", "CURVE-DEMO-RF-AMPLIFIER-GAIN")) {
                if ($curveIds -notcontains $expected) {
                    throw "Seeded engineering curve missing from API library: $expected"
                }
            }
            $recipes = Invoke-RestMethod -Uri "$agentUrl/api/v1/acquisition-channel-recipes" -TimeoutSec 10
            $recipeIds = @($recipes.items | ForEach-Object { $_.identity.entity_id })
            if ($recipeIds -notcontains "REC-DEMO-CURRENT-A") {
                throw "Seeded acquisition recipe missing from API library: REC-DEMO-CURRENT-A"
            }
        } finally {
            & $StopAgent
        }
    }

    Invoke-Step "LAB refuses non-EMC occupied port" {
        $listener = Start-DummyPortListener -Port $busyPort
        try {
            Expect-Failure "start-lab occupied by other service" {
                & $StartLab -Port $busyPort -NoBrowser -CargoCommand $CargoCommand -PythonCommand $PythonCommand
            } "health is not available"
        } finally {
            $listener.Stop()
        }
    }

    Invoke-Step "Qt Static mode uses strict JSON fixture" {
        $output = & $StartQtDemo -Mode Static -AgentUrl "http://127.0.0.1:1" -DryRun -PythonCommand $PythonCommand 2>&1
        $text = $output | Out-String
        if ($text -notlike "*Qt demo mode: Static*" -or $text -notlike "*apps\qt-console\demo\bootstrap.json*") {
            throw "Static mode did not use the Qt JSON fixture. Output: $text"
        }
    }

    Expect-Failure "Qt Agent mode fails when agent is unavailable" {
        & $StartQtDemo -Mode Agent -AgentUrl "http://127.0.0.1:1" -DryRun -PythonCommand $PythonCommand
    } "Mode Agent requested"

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
                -ArgumentList @("apps\qt-console\main.py", "--bootstrap", "apps\qt-console\demo\bootstrap.json") `
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
    & $StopAgent *> $null
}

Write-Host "Launcher smoke tests passed for $RepoRoot"
