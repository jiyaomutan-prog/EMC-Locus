param(
    [string]$NpmCommand = "npm",
    [string]$PnpmCommand = "pnpm"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$LabRoot = Join-Path $RepoRoot "apps\lab-console"
$Index = Join-Path $LabRoot "dist\index.html"

if (-not (Test-Path (Join-Path $LabRoot "package.json"))) {
    throw "LAB CONSOLE source is missing: $LabRoot"
}

Set-Location $LabRoot

if (Get-Command $NpmCommand -ErrorAction SilentlyContinue) {
    Write-Host "Building LAB CONSOLE with npm..."
    & $NpmCommand ci
    if ($LASTEXITCODE -ne 0) {
        throw "npm ci failed."
    }
    & $NpmCommand run build
    if ($LASTEXITCODE -ne 0) {
        throw "npm run build failed."
    }
} elseif (Get-Command $PnpmCommand -ErrorAction SilentlyContinue) {
    Write-Host "Building LAB CONSOLE with pnpm..."
    & $PnpmCommand install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm install --frozen-lockfile failed."
    }
    & $PnpmCommand run build
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm run build failed."
    }
} else {
    throw "Cannot rebuild LAB CONSOLE: neither '$NpmCommand' nor '$PnpmCommand' is available in PATH. Install Node.js/npm or run the release build already present under apps\lab-console\dist."
}

if (-not (Test-Path $Index)) {
    throw "LAB CONSOLE build completed without dist\index.html."
}

Write-Host "LAB CONSOLE build ready: $Index"
