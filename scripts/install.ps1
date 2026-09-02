param(
  [string]$Target = ".",
  [switch]$Force,
  [switch]$Update
)

$ErrorActionPreference = "Stop"
$Source = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
# Backups Copy-SentrithPath kept because they held files Sentrith does not
# ship (see the comment there). Reported at the end so an update never
# silently leaves an untracked directory behind, and never silently deletes
# one.
$KeptBackups = @()
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
    # A junction or directory symlink at the destination (dotfile managers,
    # a shared checkout) is refused rather than replaced -- matching the
    # file branch below, and matching install.sh. Replacing it would swap a
    # managed link for a real directory and destroy the record of where it
    # pointed. This also keeps the removal below from ever being handed a
    # reparse point: on Windows PowerShell 5.1, `Remove-Item -Recurse`
    # follows one and deletes the *link target's* contents.
    if (Test-Path $Dst) {
      $Existing = Get-Item -Force $Dst
      if ($Existing.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        Write-Error "Refusing to replace symlinked/junctioned directory: $Dst (remove it before updating the contract)"
        return
      }
    }
    # Populate a sibling stage before moving the live directory, so a failed
    # or interrupted recursive copy cannot leave the destination partial.
    $Backup = $null
    $Stage = "$Dst.sentrith-update-stage.$([Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Force -Path $Stage | Out-Null
    try {
      Get-ChildItem -Force -LiteralPath $Src | Copy-Item -Recurse -Force -Destination $Stage
      if (Test-Path $Dst) {
        $Backup = "$Dst.sentrith-update-backup.$([Guid]::NewGuid().ToString('N'))"
        Move-Item -Force $Dst $Backup
      }
      try {
        Move-Item -Force -LiteralPath $Stage -Destination $Dst
      } catch {
        if ($Backup -and -not (Test-Path -LiteralPath $Dst)) {
          Move-Item -Force -LiteralPath $Backup -Destination $Dst
          $Backup = $null
        }
        throw
      }
    } finally {
      if (Test-Path -LiteralPath $Stage) { Remove-Item -Recurse -Force -LiteralPath $Stage }
    }
    # Contract paths are Sentrith-owned and replaced wholesale, so the
    # backup is redundant *for the files Sentrith itself ships* -- but not
    # for anything else living under the same directory. This project's own
    # docs tell users to create files inside contract directories
    # (docs/rfcs/, custom .claude/skills/), and those may be uncommitted.
    # Deleting the backup unconditionally would destroy them silently;
    # keeping it unconditionally accumulates an untracked directory on every
    # update. So: remove it only when it holds nothing the new copy does
    # not, and otherwise keep it and report it.
    if ($Backup) {
      $Extra = @()
      foreach ($Item in Get-ChildItem -Recurse -Force -File $Backup) {
        $Rel = $Item.FullName.Substring($Backup.Length).TrimStart('\', '/')
        if (-not (Test-Path (Join-Path $Dst $Rel))) { $Extra += "$Path/$($Rel -replace '\\', '/')" }
      }
      if ($Extra.Count -gt 0) {
        $script:KeptBackups += "  $Backup  (contains: $($Extra -join ', '))"
      } else {
        Remove-Item -Recurse -Force $Backup
      }
    }
  } else {
    # Match the directory branch and install.sh: never hand Copy-Item a
    # managed link, and never stream a replacement directly into the live
    # contract file. A same-directory staged file keeps the existing file
    # intact until Move-Item performs the final replacement.
    $Existing = Get-Item -Force -LiteralPath $Dst -ErrorAction SilentlyContinue
    if ($null -ne $Existing -and ($Existing.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
      Write-Error "Refusing to overwrite symlinked contract file: $Dst (remove it before updating the contract)"
      return
    }
    $Temp = "$Dst.sentrith-install-file.$([Guid]::NewGuid().ToString('N'))"
    try {
      Copy-Item -Force -LiteralPath $Src -Destination $Temp
      Move-Item -Force -LiteralPath $Temp -Destination $Dst
    } finally {
      if (Test-Path -LiteralPath $Temp) { Remove-Item -Force -LiteralPath $Temp }
    }
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
  if ($KeptBackups.Count -gt 0) {
    Write-Host ("Kept backups of replaced contract directories that held files Sentrith does not ship:`n" + ($KeptBackups -join "`n"))
    Write-Host "Move anything you still need out of them, then delete them."
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
