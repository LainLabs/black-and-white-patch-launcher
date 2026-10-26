# Copies the patch's release files into payload\, which the launcher embeds at build time
# (src-tauri\build.rs), and writes payload\payload.json describing them. Nothing from the game is copied.
#
#   .\scripts\sync-payload.ps1                                  # decomp at ..\BlackWhiteDecomp, existing build
#   .\scripts\sync-payload.ps1 -Decomp E:\Projects\BlackWhiteDecomp -Build   # run its build.ps1 first
#
# The file list and the Keep/Replace rule mirror the decomp's src\setup\Payload.cpp and package.ps1.
param(
    [string] $Decomp = (Join-Path $PSScriptRoot "..\..\BlackWhiteDecomp"),
    [switch] $Build
)
$ErrorActionPreference = "Stop"

$Decomp = (Resolve-Path $Decomp).Path
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$payload = Join-Path $root "payload"

if ($Build) {
    & (Join-Path $Decomp "build.ps1") -Config Release
}

$cmake = Join-Path $Decomp "CMakeLists.txt"
$version = (Select-String -Path $cmake -Pattern 'project\(BlackWhiteDecomp VERSION ([0-9.]+)').Matches[0].Groups[1].Value
$release = Join-Path $Decomp "build\Release"
$dll = Join-Path $release "BWDecomp.dll"
if (-not (Test-Path $dll)) { throw "$dll is missing: build the patch first (or pass -Build)" }
$built = (Get-Item $dll).VersionInfo.FileVersion
if ($built -ne $version) { throw "build\Release is version $built, CMakeLists.txt says ${version}: build again (-Build)" }

# Hashes of runblack.exe the decomp recognises (src\core\Image.h), so the launcher never drifts from it.
$image = Get-Content -Raw (Join-Path $Decomp "src\core\Image.h")
function Get-Hash([string] $name) {
    $m = [regex]::Match($image, "$name\s*=\s*\""([0-9a-f]{64})\""")
    if (-not $m.Success) { throw "Cannot find $name in src\core\Image.h" }
    $m.Groups[1].Value
}
$supported = Get-Hash "kSupportedFileSha256"
$safeDisc = Get-Hash "kRetailSafeDiscFileSha256"

# target path in the game folder, source file, keep (installed only when missing, removed only when unchanged)
$files = @(
    @{ path = "BWLaunch.exe";                   source = Join-Path $release "BWLaunch.exe";                        keep = $false }
    @{ path = "BWDecomp.dll";                   source = $dll;                                                     keep = $false }
    @{ path = "BWDecomp.ini";                   source = Join-Path $Decomp "BWDecomp.ini";                         keep = $true  }
    @{ path = "BWDecomp\BWDecomp.default.ini";  source = Join-Path $Decomp "BWDecomp.ini";                         keep = $false }
    @{ path = "BWDecomp\README.txt";            source = Join-Path $Decomp "build\generated\release\README.txt";   keep = $false }
    @{ path = "BWDecomp\CHANGELOG.md";          source = Join-Path $Decomp "CHANGELOG.md";                         keep = $false }
    @{ path = "BWDecomp\FIXES.md";              source = Join-Path $Decomp "docs\FIXES.md";                        keep = $false }
    @{ path = "BWDecomp\RENDERER.md";           source = Join-Path $Decomp "docs\RENDERER.md";                     keep = $false }
    @{ path = "BWDecomp\TEXTURES.md";           source = Join-Path $Decomp "docs\TEXTURES.md";                     keep = $false }
)

Get-ChildItem $payload -Exclude ".gitkeep" | Remove-Item -Recurse -Force
$entries = foreach ($file in $files) {
    if (-not (Test-Path $file.source)) { throw "$($file.source) is missing" }
    # Stored flat under payload\files\ with the target's relative path, "\" as "/".
    $relative = $file.path -replace '\\', '/'
    $target = Join-Path $payload "files\$relative"
    New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null
    Copy-Item $file.source $target
    [ordered]@{
        path   = $file.path
        file   = "files/$relative"
        keep   = $file.keep
        sha256 = (Get-FileHash $target -Algorithm SHA256).Hash.ToLowerInvariant()
        size   = (Get-Item $target).Length
    }
}

$commit = $null
if (Get-Command git -ErrorAction SilentlyContinue) { $commit = (& git -C $Decomp rev-parse --short HEAD) }
$manifest = [ordered]@{
    decompVersion        = $version
    decompCommit         = $commit
    supportedSha256      = $supported
    retailSafeDiscSha256 = $safeDisc
    files                = @($entries)
}
$json = $manifest | ConvertTo-Json -Depth 4
[System.IO.File]::WriteAllText((Join-Path $payload "payload.json"), $json, (New-Object System.Text.UTF8Encoding $false))
Write-Host "Payload: patch $version ($commit), $($entries.Count) files in $payload"
