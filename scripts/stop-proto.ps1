$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "launcher-common.ps1")

$RepoRoot = Get-LauncherRepoRoot
$paths = Initialize-LauncherPaths -RepoRoot $RepoRoot
Stop-LauncherProcess -RuntimeRoot $paths.RuntimeRoot -Name "proto"
