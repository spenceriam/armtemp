# Probe the thermal counter classes + Qualcomm/Surface thermal driver surfaces.
$ErrorActionPreference = 'SilentlyContinue'
function Section($t) { Write-Host "`r`n==================== $t ====================" }
function Show($o) { if ($o) { $o | Format-List | Out-String -Width 300 | Write-Host } else { Write-Host '(no instances)' } }

# ---------- 1. ThermalZoneInformation perf counters (HighPerf, likely real) ----------
Section 'Win32_PerfFormattedData_Counters_ThermalZoneInformation (formatted)'
$f = Get-CimInstance -ClassName Win32_PerfFormattedData_Counters_ThermalZoneInformation
if ($f) {
    Write-Host "Count: $($f.Count)"
    $f | Select-Object Name, Temperature, HighPrecisionTemperature, PassiveLimit, Temperature_Percent, SamplePercent |
        Format-Table -AutoSize | Out-String -Width 300 | Write-Host
} else { Write-Host '(none)' }

Section 'Win32_PerfRawData_Counters_ThermalZoneInformation (raw)'
$r = Get-CimInstance -ClassName Win32_PerfRawData_Counters_ThermalZoneInformation
if ($r) {
    Write-Host "Count: $($r.Count)"
    $r | Get-Member -MemberType Property | Select-Object -ExpandProperty Name | Sort-Object | Write-Host
    Write-Host '--- values ---'
    $r | Format-List | Out-String -Width 300 | Write-Host
} else { Write-Host '(none)' }

# ---------- 2. KernelThermalConstraintChange / PolicyChange (root/wmi) ----------
Section 'KernelThermalConstraintChange (root/wmi)'
Show (Get-CimInstance -Namespace root/wmi -ClassName KernelThermalConstraintChange)

Section 'KernelThermalPolicyChange (root/wmi)'
Show (Get-CimInstance -Namespace root/wmi -ClassName KernelThermalPolicyChange)

# ---------- 3. Surface Thermal Zone Sensor Driver instances — registry params ----------
Section 'Surface Thermal Zone Sensor Driver (MSHW0188) registry detail'
$base = 'HKLM:\SYSTEM\CurrentControlSet\Enum\ACPI'
Get-ChildItem $base -ErrorAction SilentlyContinue | Where-Object { $_.PSChildName -match 'MSHW0188' } | ForEach-Object {
    $id = $_.PSChildName
    Get-ChildItem $_.PSPath | ForEach-Object {
        $p = $_.PSPath
        $d = Get-ItemProperty $p
        Write-Host ("--- {0}\{1} ---" -f $id, $_.PSChildName)
        Write-Host ("  FriendlyName: {0}" -f $d.FriendlyName)
        Write-Host ("  DeviceDesc:   {0}" -f $d.DeviceDesc)
        Write-Host ("  Class:        {0}" -f $d.Class)
        Write-Host ("  Driver:       {0}" -f $d.Driver)
        Write-Host ("  Service:      {0}" -f $d.Service)
        Write-Host ("  ConfigFlags:  {0}" -f $d.ConfigFlags)
    }
}

# ---------- 4. Qualcomm Temperature Sensor Devices registry ----------
Section 'Qualcomm Temperature Sensor Device — services/drivers'
foreach ($pid in 'QCOM0C5A','QCOM0C58','QCOM0C59','QCOM0D01','QCOM0CBF','QCOM0C91','QCOM0C5E','QCOM0C5F','QCOM0C60','QCOM0C61','QCOM0C62','QCOM0C63','QCOM0C64') {
    $k = "$base\QCOM$($pid.Substring(4))"
    # the device id is full QCOMxxxxx
    $found = Get-ChildItem $base -ErrorAction SilentlyContinue | Where-Object { $_.PSChildName -eq $pid }
    if ($found) {
        Get-ChildItem $found.PSPath | ForEach-Object {
            $d = Get-ItemProperty $_.PSPath
            Write-Host ("{0}\{1}: Service='{2}' Desc='{3}'" -f $pid, $_.PSChildName, $d.Service, ($d.DeviceDesc -replace ';\s*',' '))
        }
    }
}

# ---------- 5. Drivers by service name (find sensor driver .sys files) ----------
Section 'Sensor/thermal driver services in services key'
$svc = 'HKLM:\SYSTEM\CurrentControlSet\Services'
Get-ChildItem $svc -ErrorAction SilentlyContinue |
    Where-Object { $_.PSChildName -match 'Therm|QTI|QCOM|Sensor|Tsens|Surface' } |
    ForEach-Object {
        $d = Get-ItemProperty $_.PSPath
        Write-Host ("{0}: ImagePath='{1}' Group='{2}' Type={3} Start={4}" -f $_.PSChildName, $d.ImagePath, $d.Group, $d.Type, $d.Start)
    }

Write-Host "`r`n==================== THERMAL PROBE COMPLETE ===================="
