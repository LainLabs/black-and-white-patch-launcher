# Builds a launcher release in dist\: the NSIS setup program, the standalone (portable) executable and their
# SHA-256 sums. The update check (src-tauri\src\updates.rs) looks for exactly these asset names on GitHub releases:
#   BWLauncher-<version>-Setup.exe, BWLauncher-<version>-SHA256SUMS.txt
#
#   .\scripts\package.ps1                         # sync the payload from ..\BlackWhiteDecomp, test, build
#   .\scripts\package.ps1 -Decomp <path> -BuildDecomp   # also run the decomp's build.ps1 first
#   .\scripts\package.ps1 -SkipSync               # use payload\ as it is
#   $env:BW_LAUNCHER_UPDATE_REPO / $env:BW_PATCH_UPDATE_REPO = "owner/name"   # override a repository in updates.ini
param(
    [string] $Decomp = (Join-Path $PSScriptRoot "..\..\BlackWhiteDecomp"),
    [switch] $BuildDecomp,
    [switch] $SkipSync
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
. (Join-Path $PSScriptRoot "env.ps1")

# Native tools report progress on stderr, which Windows PowerShell turns into errors when output is redirected:
# judge them by exit code only.
function Invoke-Checked([string] $what, [scriptblock] $block) {
    $ErrorActionPreference = "Continue"
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
}

if (-not $SkipSync) {
    & (Join-Path $PSScriptRoot "sync-payload.ps1") -Decomp $Decomp -Build:$BuildDecomp
}
if (-not (Test-Path (Join-Path $root "payload\payload.json"))) { throw "payload\payload.json is missing: run without -SkipSync" }

# One version in three places.
$version = (Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content (Join-Path $root "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json).version
$cargoVersion = (Select-String -Path (Join-Path $root "src-tauri\Cargo.toml") -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
if ($version -ne $tauriVersion -or $version -ne $cargoVersion) {
    throw "Versions differ: package.json $version, tauri.conf.json $tauriVersion, Cargo.toml $cargoVersion"
}

Push-Location $root
try {
    Invoke-Checked "npm ci" { npm ci --no-audit --no-fund }
    Invoke-Checked "vp check" { npx vp check }
    Invoke-Checked "vp test" { npx vp test }
    Push-Location src-tauri
    try { Invoke-Checked "cargo test" { cargo test --release } } finally { Pop-Location }
    Invoke-Checked "tauri build" { npx tauri build }
}
finally {
    Pop-Location
}

$release = Join-Path $root "src-tauri\target\release"
$nsis = Get-ChildItem (Join-Path $release "bundle\nsis") -Filter "*_${version}_*-setup.exe" | Select-Object -First 1
if (-not $nsis) { throw "Cannot find the NSIS setup for $version in $release\bundle\nsis" }

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
$setup = Join-Path $dist "BWLauncher-$version-Setup.exe"
$portable = Join-Path $dist "BWLauncher-$version-Portable.exe"
Copy-Item $nsis.FullName $setup -Force
Copy-Item (Join-Path $release "bw-launcher.exe") $portable -Force

$sums = foreach ($file in @($setup, $portable)) {
    "$((Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant()) *$(Split-Path $file -Leaf)"
}
[System.IO.File]::WriteAllLines((Join-Path $dist "BWLauncher-$version-SHA256SUMS.txt"), [string[]] $sums)

if (Get-Command git -ErrorAction SilentlyContinue) {
    # Warnings only: git's stderr (e.g. no commits yet) must not fail a finished build.
    $ErrorActionPreference = "Continue"
    if (& git -C $root status --porcelain 2>$null) { Write-Warning "The working tree has uncommitted changes" }
    if ((& git -C $root tag --points-at HEAD 2>$null) -notcontains "v$version") { Write-Warning "HEAD is not tagged v$version" }
    $ErrorActionPreference = "Stop"
}
Get-ChildItem $dist -Filter "BWLauncher-$version-*" | Format-Table Name, Length -AutoSize
