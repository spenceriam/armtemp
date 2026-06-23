# Deep probe: hunt for ANY thermal/sensor surface on this Snapdragon X firmware.
$ErrorActionPreference = 'SilentlyContinue'
function Section($t) { Write-Host "`r`n==================== $t ====================" }

# ---------- A. Every class in root/wmi containing thermal/temp/sensor ----------
Section 'root/WMI classes matching thermal|temp|sensor|fan'
Get-CimClass -Namespace root/wmi |
    Where-Object { $_.CimClassName -match 'Thermal|Temp|Sensor|Fan' } |
    Select-Object -ExpandProperty CimClassName | Sort-Object | Write-Host

# ---------- B. Same in root/cimv2 and root/standardcimv2 ----------
foreach ($ns in 'root/cimv2','root/standardcimv2','root/hardware') {
    Section "Classes in $ns matching Thermal|Temp|Sensor|Fan|Cooling"
    $c = Get-CimClass -Namespace $ns -ErrorAction SilentlyContinue |
         Where-Object { $_.CimClassName -match 'Thermal|Temp|Sensor|Fan|Cooling' } |
         Select-Object -ExpandProperty CimClassName
    if ($c) { $c | Sort-Object | Write-Host } else { Write-Host '(none)' }
}

# ---------- C. ACPI device enumeration: find ThermalZone / temp devices ----------
Section 'ACPI Enum: device IDs containing THERMAL|TZ|TEMP|QCOM thermal'
$acpi = 'HKLM:\SYSTEM\CurrentControlSet\Enum\ACPI'
if (Test-Path $acpi) {
    Get-ChildItem $acpi | Select-Object -ExpandProperty PSChildName |
        Where-Object { $_ -match 'THERMAL|^TZ|TEMP|THRM' } | Write-Host
    Write-Host '--- QCOM devices (full list) ---'
    Get-ChildItem $acpi | Select-Object -ExpandProperty PSChildName |
        Where-Object { $_ -match 'QCOM' } | Write-Host
}

# ---------- D. Try every ACPI thermal zone instance method via WMI aside from the query ----------
Section 'MSAcpi_ThermalZoneTemperature raw (check Active/Passive trip points etc.)'
$z = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature
Write-Host ("Instance count = {0}" -f @($z).Count)
# List all properties of the class schema even if no instances
$schema = Get-CimClass -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature
if ($schema) {
    Write-Host 'Schema properties:'
    $schema.CimClassProperties | Select-Object Name, CimType | Format-Table -AutoSize | Out-String | Write-Host
} else { Write-Host 'Class not present in schema.' }

# ---------- E. Device Manager: classes under sensor / thermal / cooling ----------
Section 'PnP entities: sensor / thermal / cooling / qcom'
Get-PnpDevice -ErrorAction SilentlyContinue |
    Where-Object { $_.FriendlyName -match 'Thermal|Sensor|Temperature|Cooling|Fan|QTI|QCOM' -or $_.Class -match 'Sensor|Cooling' } |
    Select-Object Class, FriendlyName, Status, InstanceId |
    Format-Table -AutoSize | Out-String -Width 300 | Write-Host

# ---------- F. Cooling class (fans) ----------
Section 'Cooling device / fan info (root/cimv2 Win32_Fan + sensor class)'
Get-CimInstance -ClassName Win32_Fan | Format-List Name, Status
$sens = Get-CimInstance -ClassName Win32_TemperatureProbe
Write-Host ("Win32_TemperatureProbe count = {0}" -f @($sens).Count)

# ---------- G. Battery / system thermal policy hints ----------
Section 'Power / thermal policy registry'
$pp = 'HKLM:\SYSTEM\CurrentControlSet\Control\Power'
if (Test-Path $pp) {
    Get-ChildItem $pp -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.PSChildName -match 'Thermal|Trip|Temp' } |
        Select-Object -ExpandProperty Name -First 30 | Write-Host
}

Write-Host "`r`n==================== DEEP PROBE COMPLETE ===================="
