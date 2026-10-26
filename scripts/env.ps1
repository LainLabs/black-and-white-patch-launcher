# Puts cargo and the MSVC x64 tools on PATH for this PowerShell session, for machines where Rust cannot find Visual
# Studio by itself (e.g. a copy not registered with vswhere). Dot-source it: . .\scripts\env.ps1
#
# VS location: $env:BW_VS_PATH, else vswhere, else the default 2022 Build Tools / Community folders.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) { $env:Path = "$cargoBin;$env:Path" }

if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $vs = $env:BW_VS_PATH
    if (-not $vs) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswhere) { $vs = & $vswhere -latest -products * -property installationPath }
    }
    if (-not $vs) {
        $vs = @("${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools", "$env:ProgramFiles\Microsoft Visual Studio\2022\Community") |
            Where-Object { Test-Path (Join-Path $_ "VC\Auxiliary\Build\vcvars64.bat") } | Select-Object -First 1
    }
    $vcvars = Join-Path $vs "VC\Auxiliary\Build\vcvars64.bat"
    if (-not (Test-Path $vcvars)) { throw "Cannot find vcvars64.bat; set BW_VS_PATH to the Visual Studio folder" }
    # Run vcvars64 in cmd and copy the resulting environment into this session.
    # Its stderr (e.g. "vswhere is not recognized") is noise, and fatal under $ErrorActionPreference = "Stop".
    cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') { Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2] }
    }
}
