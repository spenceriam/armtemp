# Kill any leftover ARMTEMP / cargo / rustc processes that may hold the build DLL lock.
$names = 'armtemp', 'armtemp_lib', 'armtemp.exe'
$killed = 0
foreach ($n in $names) {
    Get-Process -Name $n -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("Killing leftover {0} (PID {1})" -f $_.Name, $_.Id)
        try { Stop-Process -Id $_.Id -Force; $killed++ } catch { Write-Host ("  could not kill: {0}" -f $_) }
    }
}
if ($killed -eq 0) { Write-Host "No leftover ARMTEMP processes running." }
else { Write-Host ("Killed {0} process(es)." -f $killed) }
