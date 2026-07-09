# Build the native ARM64 ARMtemp NSIS installer for local testing.
# Usage:  pwsh -File tools/build-installer.ps1
#         pwsh -File tools/build-installer.ps1 -SkipInstall   # skip npm install

param(
    [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Test-Command($name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

function Require-Command($name, $hint) {
    if (-not (Test-Command $name)) {
        throw "Missing required tool '$name'. $hint"
    }
}

Write-Host '== ARMtemp installer build ==' -ForegroundColor Cyan
Write-Host ('Root: {0}' -f $root)

# --- prerequisites ---
Require-Command node 'Install Node.js 20+: winget install OpenJS.NodeJS.20'
Require-Command npm  'Node.js npm should ship with Node.js'
Require-Command cargo 'Install Rust: winget install Rustlang.Rustup, then restart the shell'

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($arch -ne 'Arm64') {
    Write-Warning ('Host OS is {0}, not Arm64. ARMtemp targets aarch64-pc-windows-msvc.' -f $arch)
}

$rustHost = (& rustup show active-toolchain 2>$null | Select-Object -First 1)
Write-Host ('Rust:    {0}' -f $rustHost)
Write-Host ('Node:    {0}' -f (& node --version))
Write-Host ('npm:     {0}' -f (& npm --version))

# MSVC link.exe is not on PATH by default — initialize the ARM64 dev environment.
$vcvars = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
if (-not (Test-Path $vcvars)) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $vsRoot = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath 2>$null
        if ($vsRoot) {
            $vcvars = Join-Path $vsRoot 'VC\Auxiliary\Build\vcvarsall.bat'
        }
    }
}
if (-not (Test-Path $vcvars)) {
    throw @"
MSVC ARM64 build tools not found.
Install (or finish installing) Visual Studio 2022 Build Tools with the C++ ARM64 workload, then reboot if prompted:
  winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.ARM64 --includeRecommended"
"@
}

$envDump = cmd.exe /c "`"$vcvars`" arm64 >nul 2>&1 && set"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to initialize MSVC environment via vcvarsall.bat arm64"
}
foreach ($line in $envDump) {
    if ($line -match '^(?<key>[^=]+)=(?<val>.*)$') {
        Set-Item -Path "Env:$($Matches.key)" -Value $Matches.val
    }
}
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    throw @"
link.exe still not on PATH after vcvarsall arm64.
If you just installed VS Build Tools, reboot Windows first — the installer often requires it before the ARM64 linker is available.
"@
}
Write-Host ('MSVC:    {0}' -f (Get-Command link.exe).Source)

if (-not $SkipInstall) {
    Write-Host ''
    Write-Host 'Installing frontend dependencies...' -ForegroundColor Yellow
    npm install
}

Write-Host ''
Write-Host 'Building NSIS installer...' -ForegroundColor Yellow
npm run tauri build

$bundleRoot = Join-Path $root 'src-tauri\target\release\bundle'
$exe = Join-Path $root 'src-tauri\target\release\armtemp.exe'

if (-not (Test-Path $exe)) {
    throw "Build failed: $exe not found"
}

# Verify ARM64 PE machine type (0xAA64).
$bytes = [System.IO.File]::ReadAllBytes($exe)
$peOffset = [BitConverter]::ToInt32($bytes, 60)
$machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
if ($machine -ne 0xAA64) {
    throw ('Expected ARM64 (0xAA64) binary, got machine type 0x{0:X4}' -f $machine)
}

Write-Host ''
Write-Host '== Build outputs ==' -ForegroundColor Green
Write-Host ('EXE:  {0} ({1:N2} MB, ARM64)' -f $exe, ($bytes.Length / 1MB))

$nsisDir = Join-Path $bundleRoot 'nsis'
if (Test-Path $nsisDir) {
    Get-ChildItem $nsisDir -File | ForEach-Object {
        Write-Host ('NSIS: {0} ({1:N2} MB)' -f $_.FullName, ($_.Length / 1MB))
    }
}

Write-Host ''
Write-Host 'To install locally, run the NSIS setup.exe.' -ForegroundColor Cyan
