param(
    [switch]$SkipE2E,
    [switch]$SkipSmoke,
    [switch]$NoInstall
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Invoke-External {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = $RepoRoot
    )

    Write-Host ""
    Write-Host "== $Label"
    Write-Host ">> $FilePath $($Arguments -join ' ')"
    Push-Location $WorkingDirectory
    try {
        & $FilePath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Label failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }
}

function Invoke-PowerShellScript {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [string[]]$Arguments = @()
    )

    Invoke-External `
        -Label $Label `
        -FilePath "powershell" `
        -Arguments (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath) + $Arguments)
}

Push-Location $RepoRoot
try {
    Write-Host "EMC Locus CI validation"
    Write-Host "Repository: $RepoRoot"

    Invoke-External -Label "Rust compiler version" -FilePath "rustc" -Arguments @("--version")
    Invoke-External -Label "Cargo version" -FilePath "cargo" -Arguments @("--version")
    Invoke-External -Label "Python launcher version" -FilePath "py" -Arguments @("--version")
    Invoke-External -Label "Node version" -FilePath "node" -Arguments @("--version")
    Invoke-External -Label "npm version" -FilePath "npm" -Arguments @("--version")

    Invoke-External -Label "Check Rust formatting" -FilePath "cargo" -Arguments @("fmt", "--check")
    Invoke-External -Label "Run Clippy" -FilePath "cargo" -Arguments @("clippy", "--workspace", "--all-targets", "--", "-D", "warnings")
    Invoke-External -Label "Run Rust tests" -FilePath "cargo" -Arguments @("test", "--workspace")

    Invoke-External -Label "Compile Python package" -FilePath "py" -Arguments @("-m", "compileall", "python\emc_locus")
    Invoke-External -Label "Compile Qt console" -FilePath "py" -Arguments @("-m", "py_compile", "apps\qt-console\main.py")

    $env:PYTHONPATH = "python"
    Invoke-External -Label "Run Python tests" -FilePath "py" -Arguments @("-m", "unittest", "discover", "-s", "python\tests")
    Invoke-External `
        -Label "Validate SQLite migrations" `
        -FilePath "py" `
        -Arguments @("-c", "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))")

    $LabRoot = Join-Path $RepoRoot "apps\lab-console"
    if (-not $NoInstall) {
        Invoke-External -Label "Install LAB CONSOLE dependencies" -FilePath "npm" -Arguments @("ci") -WorkingDirectory $LabRoot
    } else {
        Write-Host ""
        Write-Host "== Install LAB CONSOLE dependencies"
        Write-Host ">> skipped because -NoInstall was supplied"
    }

    Invoke-External -Label "Typecheck LAB CONSOLE" -FilePath "npm" -Arguments @("run", "typecheck") -WorkingDirectory $LabRoot
    Invoke-External -Label "Lint LAB CONSOLE" -FilePath "npm" -Arguments @("run", "lint") -WorkingDirectory $LabRoot
    Invoke-External -Label "Test LAB CONSOLE units" -FilePath "npm" -Arguments @("run", "test") -WorkingDirectory $LabRoot
    Invoke-External -Label "Build LAB CONSOLE" -FilePath "npm" -Arguments @("run", "build") -WorkingDirectory $LabRoot
    Invoke-External -Label "Verify versioned LAB CONSOLE dist" -FilePath "git" -Arguments @("diff", "--exit-code", "--", "apps/lab-console/dist")

    if (-not $SkipE2E) {
        if (-not $NoInstall) {
            Invoke-External -Label "Install Playwright browser" -FilePath "npx" -Arguments @("playwright", "install", "chromium") -WorkingDirectory $LabRoot
        } else {
            Write-Host ""
            Write-Host "== Install Playwright browser"
            Write-Host ">> skipped because -NoInstall was supplied"
        }
        Invoke-External -Label "Run LAB CONSOLE E2E" -FilePath "npm" -Arguments @("run", "test:e2e") -WorkingDirectory $LabRoot
    } else {
        Write-Host ""
        Write-Host "== Run LAB CONSOLE E2E"
        Write-Host ">> skipped because -SkipE2E was supplied"
    }

    $env:PYTHONPATH = "python"
    Invoke-External -Label "Check release consistency" -FilePath "py" -Arguments @("-m", "unittest", "python.tests.test_release_consistency")

    if (-not $SkipSmoke) {
        Invoke-PowerShellScript -Label "Run launcher smoke" -ScriptPath (Join-Path $RepoRoot "scripts\smoke-launchers.ps1") -Arguments @("-SkipQtOffscreen")
    } else {
        Write-Host ""
        Write-Host "== Run launcher smoke"
        Write-Host ">> skipped because -SkipSmoke was supplied"
    }

    Invoke-External -Label "Check whitespace" -FilePath "git" -Arguments @("diff", "--check")
    Invoke-External -Label "Check staged whitespace" -FilePath "git" -Arguments @("diff", "--cached", "--check")

    Write-Host ""
    Write-Host "EMC Locus CI validation passed."
} finally {
    Pop-Location
}
