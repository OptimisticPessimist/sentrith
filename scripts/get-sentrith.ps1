# Download the prebuilt sentrith.exe for Windows into <Target>\bin\,
# verifying it against the release's SHA256SUMS.
#
# Usage:
#   ./scripts/get-sentrith.ps1 [-Target .] [-Tag sentrith-vX.Y.Z] [-Repo owner/name]
#
# Defaults: Target=. , Repo=OptimisticPessimist/sentrith, Tag=newest sentrith-v* release.

param(
  [string]$Target = ".",
  [string]$Tag = "",
  [string]$Repo = "OptimisticPessimist/sentrith"
)

$ErrorActionPreference = "Stop"

if ($env:SENTRITH_REPO) { $Repo = $env:SENTRITH_REPO }

$Asset = "sentrith-windows-x86_64.exe"
$BinName = "sentrith.exe"

if (-not $Tag) {
  $Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases?per_page=30"
  $Match = $Releases | Where-Object { $_.tag_name -like "sentrith-v*" } | Select-Object -First 1
  if (-not $Match) { Write-Error "SENTRITH-GET: no sentrith-v* release found in $Repo" }
  $Tag = $Match.tag_name
}

$Base = "https://github.com/$Repo/releases/download/$Tag"
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("sentrith-get-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null

try {
  Write-Host "SENTRITH-GET: downloading $Asset from $Tag"
  Invoke-WebRequest -Uri "$Base/$Asset" -OutFile (Join-Path $Tmp $Asset)
  Invoke-WebRequest -Uri "$Base/SHA256SUMS" -OutFile (Join-Path $Tmp "SHA256SUMS")

  $Line = (Get-Content (Join-Path $Tmp "SHA256SUMS")) | Where-Object { $_ -match "\s$([regex]::Escape($Asset))$" } | Select-Object -First 1
  if (-not $Line) { Write-Error "SENTRITH-GET: $Asset not listed in SHA256SUMS" }
  $Expected = ($Line -split "\s+")[0].ToLower()
  $Actual = (Get-FileHash -Algorithm SHA256 (Join-Path $Tmp $Asset)).Hash.ToLower()
  if ($Expected -ne $Actual) { Write-Error "SENTRITH-GET: checksum mismatch for $Asset; refusing to install" }

  $BinDir = Join-Path (Resolve-Path $Target).Path "bin"
  New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
  Copy-Item -Force (Join-Path $Tmp $Asset) (Join-Path $BinDir $BinName)
  Write-Host "SENTRITH-GET: installed $(Join-Path $BinDir $BinName) ($Tag, sha256 verified)"
}
finally {
  Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
