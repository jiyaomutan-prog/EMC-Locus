param(
    [string]$NpmCommand = "npm"
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

if (-not (Get-Command $NpmCommand -ErrorAction SilentlyContinue)) {
    throw "Cannot rebuild LAB CONSOLE: '$NpmCommand' is not available in PATH. Install Node.js/npm or run the release build already present under apps\lab-console\dist."
}

Write-Host "Building LAB CONSOLE with npm..."
& $NpmCommand ci
if ($LASTEXITCODE -ne 0) {
    throw "npm ci failed."
}
& $NpmCommand run build
if ($LASTEXITCODE -ne 0) {
    throw "npm run build failed."
}

if (-not (Test-Path $Index)) {
    throw "LAB CONSOLE build completed without dist\index.html."
}

Write-Host "LAB CONSOLE build ready: $Index"
