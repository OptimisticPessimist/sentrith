param(
  [string]$Target = ".",
  [switch]$Force,
  [switch]$Update
)

$ErrorActionPreference = "Stop"
$Source = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TargetPath = (Resolve-Path $Target).Path

# Contract paths are owned by Sentrith and are replaced on update.
$ContractPaths = @(
  "AGENTS.md",
  "CLAUDE.md",
  ".github/copilot-instructions.md",
  ".github/prompts",
  ".agents",
  ".claude/skills",
  ".claude/settings.hooks.example.json",
  ".codex/hooks.example.json",
  "docs/ai/BOOTSTRAP.md",
  "docs/ai/MEMORY_POLICY.md",
  "docs/ai/MEMORY_AUDIT.md",
  "docs/ai/PRIVATE_CONTEXT.md",
  "docs/ai/TASK_PROTOCOL.md",
  "docs/ai/TASK_CLOSEOUT.md",
  "docs/development",
  "docs/profiles",
  "docs/specs/README.md",
  "docs/specs/_templates",
  "docs/rfcs"
)

# Seed paths are owned by the project after installation. They are created when
# missing and never overwritten, so project memory survives an update.
$SeedPaths = @(
  "docs/ai/PROJECT.md",
  "docs/ai/STATE.md",
  "docs/ai/PROFILE.md",
  "docs/ai/DECISIONS.md",
  "docs/ai/KNOWN_ISSUES.md"
)

function Copy-SentrithPath {
  param([string]$Path)
  $Src = Join-Path $Source $Path
  $Dst = Join-Path $TargetPath $Path
  if (-not (Test-Path $Src)) { return }
  New-Item -ItemType Directory -Force -Path (Split-Path $Dst -Parent) | Out-Null
  if ((Get-Item $Src).PSIsContainer) {
    New-Item -ItemType Directory -Force -Path $Dst | Out-Null
    Copy-Item -Recurse -Force (Join-Path $Src "*") $Dst
  } else {
    Copy-Item -Force $Src $Dst
  }
}

if ($Update) {
  foreach ($Path in $ContractPaths) { Copy-SentrithPath $Path }

  $Added = @()
  foreach ($Path in $SeedPaths) {
    if (-not (Test-Path (Join-Path $TargetPath $Path))) {
      Copy-SentrithPath $Path
      $Added += $Path
    }
  }

  Write-Host "Sentrith contract updated in: $TargetPath"
  Write-Host "Project-owned memory was not modified."
  if ($Added.Count -gt 0) {
    Write-Host ("New memory files added as uninitialized templates:`n  " + ($Added -join "`n  "))
    Write-Host "Ask your agent to fill them (see docs/ai/BOOTSTRAP.md)."
  }
  Write-Host "Review: git diff -- AGENTS.md docs/development docs/profiles"
  Write-Host "Post-update steps: docs/guide/UPDATING.en.md (日本語: UPDATING.ja.md) in the Sentrith source."
  exit 0
}

# Fresh install: refuse to clobber anything already present.
if (-not $Force) {
  $Conflicts = @()
  foreach ($Path in ($ContractPaths + $SeedPaths)) {
    if (Test-Path (Join-Path $TargetPath $Path)) { $Conflicts += $Path }
  }
  if ($Conflicts.Count -gt 0) {
    Write-Error ("Sentrith install stopped because target already contains:`n  " + ($Conflicts -join "`n  ") + "`nIf Sentrith is already installed, rerun with -Update: it replaces contract files and preserves project memory.`nUse -Force only when full replacement is intentional; it overwrites project memory.")
  }
}

foreach ($Path in ($ContractPaths + $SeedPaths)) { Copy-SentrithPath $Path }

Write-Host "Sentrith repository contract copied to: $TargetPath"
Write-Host "Next:"
Write-Host "1. Ask your coding agent to read docs/ai/BOOTSTRAP.md and bootstrap Project Memory."
Write-Host "2. Review docs/ai/PROJECT.md, docs/ai/STATE.md, and docs/ai/PROFILE.md once."
Write-Host "3. Optional: ./scripts/get-sentrith.ps1 -Target $TargetPath  (downloads the prebuilt CLI into bin/)"
Write-Host "4. Run 'sentrith preflight' if the CLI is installed."
