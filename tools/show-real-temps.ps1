# Show the REAL temperatures ARMTEMP reads (same probe the Rust backend uses).
$ErrorActionPreference = 'SilentlyContinue'
$zones = Get-CimInstance Win32_PerfFormattedData_Counters_ThermalZoneInformation |
    Select-Object Name, Temperature, HighPrecisionTemperature, PercentPassiveLimit |
    Where-Object { $_.HighPrecisionTemperature -gt 2500 -or $_.Temperature -gt 250 }
Write-Host ('=== REAL live thermal zones ({0}) — what ARMTEMP displays ===' -f $zones.Count)
$zones | Sort-Object HighPrecisionTemperature -Descending |
    Select-Object @{N='Zone';E={$_.Name}}, @{N='Celsius';E={[math]::Round($_.HighPrecisionTemperature/10 - 273.15,1)}}, @{N='Throttled';E={$_.PercentPassiveLimit -lt 100}} |
    Format-Table -AutoSize | Out-String -Width 120 | Write-Host

$cpu = Get-CimInstance Win32_Processor
Write-Host ('Package (hottest zone): ' + [math]::Round((($zones | Measure-Object HighPrecisionTemperature -Maximum).Maximum)/10 - 273.15,1) + ' C')
Write-Host ('CPU: ' + $cpu.Name + '  cur ' + $cpu.CurrentClockSpeed + ' MHz')
