param(
    [string]$StateName = "agent"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
if ($StateName -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') {
    throw "StateName contains unsupported characters: $StateName"
}
Stop-LauncherProcess -RuntimeRoot $paths.RuntimeRoot -Name $StateName
