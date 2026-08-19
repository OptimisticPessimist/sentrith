param(
  [string]$Target = ".",
  [switch]$Force
)

$ErrorActionPreference = "Stop"
$Source = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TargetPath = (Resolve-Path $Target).Path

$Paths = @(
  "AGENTS.md",
  "CLAUDE.md",
  ".github/copilot-instructions.md",
  ".github/prompts",
  ".claude",
  ".codex",
  "docs/ai",
  "docs/development",
  "docs/specs",
  "docs/rfcs"
)

if (-not $Force) {
  $Conflicts = @()
  foreach ($Path in $Paths) {
    if (Test-Path (Join-Path $TargetPath $Path)) { $Conflicts += $Path }
  }
  if ($Conflicts.Count -gt 0) {
    Write-Error ("Sentrith install stopped because target already contains:`n  " + ($Conflicts -join "`n  ") + "`nReview/merge manually, or rerun with -Force only if replacement is intentional.")
  }
}

foreach ($Path in $Paths) {
  $Src = Join-Path $Source $Path
  $Dst = Join-Path $TargetPath $Path
  if (-not (Test-Path $Src)) { continue }
  New-Item -ItemType Directory -Force -Path (Split-Path $Dst -Parent) | Out-Null
  if ((Get-Item $Src).PSIsContainer) {
    New-Item -ItemType Directory -Force -Path $Dst | Out-Null
    Copy-Item -Recurse -Force (Join-Path $Src "*") $Dst
  } else {
    Copy-Item -Force $Src $Dst
  }
}

Write-Host "Sentrith repository contract copied to: $TargetPath"
Write-Host "Next:"
Write-Host "1. Ask your coding agent to read docs/ai/BOOTSTRAP.md and bootstrap Project Memory."
Write-Host "2. Review docs/ai/PROJECT.md and docs/ai/STATE.md once."
Write-Host "3. Run 'sentrith preflight' if the CLI is installed."
