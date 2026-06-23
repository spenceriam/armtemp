# ARMTEMP sensor discovery probe — read-only investigation of what real telemetry
# this Snapdragon X (X1P64100) firmware exposes. Output goes to stdout; capture to SENSORS.md.

$ErrorActionPreference = 'SilentlyContinue'

function Section($t) { Write-Host "`r`n==================== $t ====================" }

# ---------- 1. ACPI thermal zones (the most promising source) ----------
Section 'ACPI ThermalZone — MSAcpi_ThermalZoneTemperature (root/wmi)'
$zones = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature
if (-not $zones) {
    Write-Host 'NO RESULTS: class returned nothing.'
} else {
    foreach ($z in $zones) {
        $celsius = if ($z.CurrentTemperature -gt 0) { [math]::Round($z.CurrentTemperature / 10 - 273.15, 1) } else { $null }
        Write-Host ("InstanceName={0}  CurrentTemperature={1}  -> {2} C" -f $z.InstanceName, $z.CurrentTemperature, $celsius)
    }
}

# ---------- 2. Refresh check: re-query twice with a delay, see if value changes ----------
Section 'ACPI refresh check (re-query after 2s)'
$z1 = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature
Start-Sleep -Seconds 2
$z2 = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature
if ($z1 -and $z2) {
    for ($i = 0; $i -lt $z1.Count; $i++) {
        $a = $z1[$i].CurrentTemperature; $b = $z2[$i].CurrentTemperature
        Write-Host ("{0}: first={1} second={2} delta={3}" -f $z1[$i].InstanceName, $a, $b, ($b - $a))
    }
} else { Write-Host 'Cannot refresh-check (no zones).' }

# ---------- 3. Win32_TemperatureProbe (usually empty) ----------
Section 'Win32_TemperatureProbe'
$tp = Get-CimInstance -ClassName Win32_TemperatureProbe
if (-not $tp) { Write-Host 'NO RESULTS (expected — rarely populated).' }
else { $tp | Format-List Name, CurrentReading, Description }

# ---------- 4. LibreHardwareMonitor / OpenHardwareMonitor namespaces ----------
Section 'Third-party monitor WMI namespaces'
foreach ($ns in 'root/LibreHardwareMonitor','root/OpenHardwareMonitor') {
    $test = Get-CimInstance -Namespace $ns -ClassName __Namespace -ErrorAction SilentlyContinue
    if ($test) { Write-Host "$ns : EXISTS" } else { Write-Host "$ns : absent" }
}

# ---------- 5. Per-core load / clock (always available via perf counters) ----------
Section 'Per-core load (Win32_PerfFormattedData PerfOS_Processor)'
$proc = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfOS_Processor | Where-Object Name -ne '_Total'
$proc | Select-Object Name, PercentProcessorTime, ProcessorFrequency, PercentProcessorPerformance |
    Format-Table -AutoSize | Out-String -Width 200 | Write-Host

# ---------- 6. Processor static info ----------
Section 'Processor static'
$cpu = Get-CimInstance Win32_Processor
Write-Host ("Name={0}" -f $cpu.Name)
Write-Host ("Cores={0} Logical={1} MaxClock={2} CurClock={3}" -f $cpu.NumberOfCores, $cpu.NumberOfLogicalProcessors, $cpu.MaxClockSpeed, $cpu.CurrentClockSpeed)
Write-Host ("ProcessorId/CPUStatus/LoadPct={0}/{1}/{2}" -f $cpu.ProcessorId, $cpu.CpuStatus, $cpu.LoadPercentage)

# ---------- 7. Qualcomm / OEM WMI namespaces search ----------
Section 'All WMI namespaces under root (look for qcom/qualcomm/oem/sensor/thermal)'
$namespaces = Get-CimInstance -Namespace root -ClassName __Namespace | Select-Object -ExpandProperty Name
Write-Host ("Root namespaces: {0}" -f ($namespaces -join ', '))
$interesting = $namespaces | Where-Object { $_ -match 'qcom|qualcomm|oem|sensor|thermal|qti' }
if ($interesting) { Write-Host ("INTERESTING: {0}" -f ($interesting -join ', ')) } else { Write-Host 'No obviously OEM/thermal-named namespaces at root.' }

# ---------- 8. Registry hints for thermal / sensor class names ----------
Section 'Registry: sensor/thermal class GUIDs (Enum branches)'
$keys = @(
  'HKLM:\SYSTEM\CurrentControlSet\Control\Class',
  'HKLM:\SYSTEM\CurrentControlSet\Enum\ACPI'
)
foreach ($k in $keys) {
    if (Test-Path $k) {
        Get-ChildItem $k -ErrorAction SilentlyContinue |
            Where-Object { $_.PSChildName -match 'thermal|sensor|qcom|qti|temp|acpi' -or $_.Name -match 'ThermalZone' } |
            Select-Object -ExpandProperty PSChildName -First 40 | Write-Host
    }
}

Write-Host "`r`n==================== PROBE COMPLETE ===================="
