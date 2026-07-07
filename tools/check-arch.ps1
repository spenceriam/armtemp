# Reports the PE machine type of the built ARMTEMP executable.
$exe = Join-Path $PSScriptRoot '..\src-tauri\target\release\armtemp.exe'
$bytes = [System.IO.File]::ReadAllBytes($exe)
$peOffset = [BitConverter]::ToInt32($bytes, 60)
$machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
$arch = switch ($machine) {
    0xAA64 { 'AA64 (ARM64 / aarch64)' }
    0x8664 { '8664 (x64)' }
    0x014C { '14C (x86)' }
    default { ('0x{0:X4} (unknown)' -f $machine) }
}
Write-Host ('File:      {0}' -f $exe)
Write-Host ('Machine:   {0}' -f $arch)
Write-Host ('Size:      {0:N0} bytes ({1:N2} MB)' -f $bytes.Length, ($bytes.Length / 1MB))
