param(
    [string]$Baseline = "c4e361b17fc37cb22ca209eb0d3f0d238b70673c"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$HistoricalPaths = @(
    "docs/ux/0.18.0/screenshots",
    "docs/ux/0.19.0/screenshots",
    "docs/ux/0.20.0/screenshots",
    "docs/ux/0.21.0/screenshots"
)

Push-Location $RepoRoot
try {
    & git cat-file -e "$Baseline`^{commit}"
    if ($LASTEXITCODE -ne 0) {
        throw "Historical screenshot baseline $Baseline is unavailable. Fetch full Git history before validation."
    }

    $Arguments = @("diff", "--exit-code", $Baseline, "--") + $HistoricalPaths
    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Historical release screenshots differ from baseline $Baseline."
    }

    Write-Host "Historical release screenshots match baseline $Baseline."
} finally {
    Pop-Location
}
