# Launch the built ARMTEMP app, confirm it runs, then stop it.
$exe = Join-Path $PSScriptRoot '..\src-tauri\target\release\armtemp.exe'
Write-Host 'Launching ARMTEMP...'
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 5

$running = Get-Process -Id $p.Id -ErrorAction SilentlyContinue
if ($running) {
    Write-Host ('RUNNING: PID {0}, working set {1:N0} MB' -f $running.Id, ($running.WorkingSet64 / 1MB))
} else {
    Write-Host 'Process exited (check stderr).'
    Start-Sleep -Seconds 1
}

# Clean stop.
Get-Process -Name armtemp -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host 'Stopped.'
