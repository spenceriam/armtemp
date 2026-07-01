# verify-temps.ps1 -- independent cross-check of ARMTEMP's readings.
#
# Reads the SAME Windows performance counters ARMTEMP's native PDH backend
# uses, applies the SAME transformation rules (documented in SENSORS.md and
# src-tauri/src/sensors/pdh.rs), and prints the per-core table the app should
# be showing. Run it side-by-side with ARMTEMP: every row should match the
# app's Temperature Readings within one polling tick (~1.5 s) / 1 degC.
#
# Transformation rules mirrored from the app:
#   1. Zone temp = High Precision Temperature (tenths of Kelvin) when > 0,
#      else Temperature (Kelvin) * 10.
#   2. Valid zones only: >= 2730 tenths-K (drops the -40C / -18C sentinels
#      and 0-inactive zones).
#   3. degC = tenthsK / 10 - 273.15.
#   4. Core mapping: zones sorted hottest-first; Core #0 = hottest zone,
#      Core #1 = next, ...; cores beyond the zone count reuse the coolest.
#      (This is a zone->core mapping, NOT true per-core sensors -- Snapdragon X
#      firmware exposes no per-core surface; see SENSORS.md section 5.)
#   5. Package = hottest valid zone. Average = mean of the mapped core temps.

$ErrorActionPreference = "Stop"

# --- gather the raw counters (one sample set) ---
$hp    = (Get-Counter '\Thermal Zone Information(*)\High Precision Temperature').CounterSamples
$whole = (Get-Counter '\Thermal Zone Information(*)\Temperature').CounterSamples
$loads = (Get-Counter '\Processor Information(*)\% Processor Time').CounterSamples |
         Where-Object { $_.InstanceName -notmatch '_total' }
$freq  = (Get-Counter '\Processor Information(_Total)\Processor Frequency').CounterSamples[0].CookedValue

# --- zone temps with the app's exact rules ---
$zones = foreach ($s in $whole) {
  $name = $s.InstanceName
  $hpv  = ($hp | Where-Object InstanceName -eq $name | Select-Object -First 1).CookedValue
  $tenthsK = if ($hpv -gt 0) { $hpv } else { $s.CookedValue * 10 }
  if ($tenthsK -lt 2730) { continue }   # sentinel / inactive -> dropped, never shown
  [pscustomobject]@{ Zone = $name; TempC = [math]::Round($tenthsK / 10 - 273.15, 1) }
}
$zones = @($zones | Sort-Object TempC -Descending)

# --- per-core loads keyed by core index ("group,core" instances) ---
$loadByCore = @{}
foreach ($l in $loads) {
  $idx = [int]($l.InstanceName -split ',')[-1]
  $loadByCore[$idx] = [math]::Round($l.CookedValue)
}
$coreCount = $loadByCore.Keys.Count
$coolest = $zones[-1].TempC

Write-Output ""
Write-Output "=== ARMTEMP self-check: expected app values from raw counters ==="
Write-Output ("Live frequency : {0:N2} GHz" -f ($freq / 1000))
Write-Output ("Valid zones    : {0} (of {1} instances)" -f $zones.Count, $whole.Count)
Write-Output ("Package (max)  : {0} degC   <- app's 'CPU Temp' / hottest zone" -f $zones[0].TempC)
Write-Output ""
Write-Output "Core  ExpectedTemp  SourceZone       Load"
$sum = 0.0
for ($i = 0; $i -lt $coreCount; $i++) {
  if ($i -lt $zones.Count) { $z = $zones[$i] } else { $z = [pscustomobject]@{ Zone = "(coolest reused)"; TempC = $coolest } }
  $sum += $z.TempC
  "Core #{0,-2} {1,6} degC   {2,-15} {3,3} %" -f $i, $z.TempC, $z.Zone, $loadByCore[$i]
}
Write-Output ""
Write-Output ("Average        : {0} degC   <- app's 'Avg' / tray (default mode)" -f [math]::Round($sum / $coreCount, 0))
Write-Output ""
Write-Output "Compare each row against ARMTEMP's Temperature Readings. Values move"
Write-Output "every poll tick, so expect agreement within ~1 tick / ~1 degC."
Write-Output "Independent reference: HWiNFO64 (native ARM64 build) reads Snapdragon X"
Write-Output "sensors via its own path -- its CPU zone temps should track these too."
