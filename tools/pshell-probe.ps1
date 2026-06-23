# Confirm the exact PowerShell-CIM invocation we'll shell out to from Rust.
$ErrorActionPreference = 'SilentlyContinue'
$out = [ordered]@{
  cpu = Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors,MaxClockSpeed,CurrentClockSpeed
  zones = Get-CimInstance Win32_PerfFormattedData_Counters_ThermalZoneInformation | Select-Object Name,Temperature,HighPrecisionTemperature,PercentPassiveLimit
  cores = Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor | Where-Object Name -ne '_Total' | Select-Object Name,PercentProcessorTime
}
# Single line so Rust can parse one JSON document.
$j = $out | ConvertTo-Json -Depth 4 -Compress
Write-Output $j
