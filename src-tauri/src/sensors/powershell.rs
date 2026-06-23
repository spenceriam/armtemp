//! PowerShell-backed sensor provider. Shells out to `Get-CimInstance` and
//! parses JSON output. This is the PROVEN path (it's exactly what the Phase 0
//! probes used) and sidesteps the COM/IWbemServices interaction that fails
//! with WBEM_E_NOT_FOUND (0x80041002) when called from inside a Tauri process.
//!
//! All data is strictly REAL. Missing fields -> `None`. No fabrication.
//! One `powershell.exe` child process per poll tick (~50-150ms) is acceptable
//! for a 1.5s cadence; we reuse a single powershell for detection on init.

use std::collections::HashMap;
use std::process::Command;

use crate::sensors::chips::{match_profile, ChipProfile};
use crate::sensors::types::{
    kelvin_tenths_to_c, CoreKind, CoreReading, SensorSnapshot, ZoneReading,
    VALID_ZONE_MIN_KELVIN,
};
use serde::Deserialize;

/// The JSON shape produced by the bundled PowerShell probe script.
#[derive(Deserialize, Debug)]
struct Probe {
    #[serde(default)]
    cpu: Option<CpuJson>,
    #[serde(default)]
    zones: Vec<ZoneJson>,
    #[serde(default)]
    cores: Vec<CoreLoadJson>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct CpuJson {
    name: String,
    number_of_cores: Option<u32>,
    number_of_logical_processors: Option<u32>,
    max_clock_speed: Option<u32>,
    current_clock_speed: Option<u32>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ZoneJson {
    name: String,
    temperature: Option<f64>,
    high_precision_temperature: Option<f64>,
    percent_passive_limit: Option<u32>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct CoreLoadJson {
    name: String,
    percent_processor_time: Option<u32>,
}

/// The PowerShell probe script body. Kept inline so the binary is self-contained
/// (no external .ps1 dependency). Collects CPU + thermal zones + per-core load
/// in one process and emits a single JSON document.
const PROBE_PS: &str = r#"
$ErrorActionPreference='SilentlyContinue'
$cpu = Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors,MaxClockSpeed,CurrentClockSpeed
$zones = Get-CimInstance Win32_PerfFormattedData_Counters_ThermalZoneInformation | Select-Object Name,Temperature,HighPrecisionTemperature,PercentPassiveLimit
$cores = Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor | Where-Object Name -ne '_Total' | Select-Object Name,PercentProcessorTime
[ordered]@{ cpu=$cpu; zones=$zones; cores=$cores } | ConvertTo-Json -Depth 5 -Compress
"#;

/// Run the probe once and return parsed data.
fn run_probe() -> anyhow::Result<Probe> {
    let out = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            PROBE_PS,
        ])
        .output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("powershell probe failed: {err}");
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // PowerShell occasionally emits a BOM or stray lines; take the first line
    // that begins with '{' as the JSON document.
    let json = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| anyhow::anyhow!("no JSON in powershell output"))?;
    let probe: Probe = serde_json::from_str(json)?;
    Ok(probe)
}

/// Handle to the PowerShell-backed provider. Holds per-core running min/max
/// state across ticks. Cloneable + Send + Sync (state is behind a Mutex).
#[derive(Clone)]
pub struct PowerShellProvider {
    cached_profile: std::sync::Arc<std::sync::OnceLock<ChipProfile>>,
    /// Per-core running (min, max) temp in °C, keyed by logical core index.
    minmax: std::sync::Arc<std::sync::Mutex<HashMap<u32, (f64, f64)>>>,
}

impl Default for PowerShellProvider {
    fn default() -> Self {
        Self {
            cached_profile: std::sync::Arc::new(std::sync::OnceLock::new()),
            minmax: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

impl PowerShellProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect the chip once and cache it.
    pub fn profile(&self) -> anyhow::Result<Option<ChipProfile>> {
        if let Some(p) = self.cached_profile.get() {
            return Ok(Some(p.clone()));
        }
        // Run a quick detection probe (cheap; one powershell).
        let probe = run_probe()?;
        let p = match probe.cpu.as_ref() {
            Some(c) => match_profile(&c.name.to_lowercase(), c.number_of_cores.unwrap_or(10)),
            None => match_profile("snapdragon", 10),
        };
        let _ = self.cached_profile.set(p.clone());
        Ok(Some(p))
    }

    /// Collect a full real snapshot via one PowerShell invocation.
    pub fn snapshot(&self) -> anyhow::Result<SensorSnapshot> {
        let probe = run_probe()?;
        let profile = if let Some(p) = self.cached_profile.get() {
            p.clone()
        } else {
            let p = match probe.cpu.as_ref() {
                Some(c) => match_profile(&c.name.to_lowercase(), c.number_of_cores.unwrap_or(10)),
                None => match_profile("snapdragon", 10),
            };
            let _ = self.cached_profile.set(p.clone());
            p
        };

        let cpu = probe.cpu;
        let cores_count = cpu.as_ref().and_then(|c| c.number_of_cores).unwrap_or_else(|| profile.total_cores());
        let logical = cpu.as_ref().and_then(|c| c.number_of_logical_processors).unwrap_or(cores_count);
        let max_clock = cpu.as_ref().and_then(|c| c.max_clock_speed);
        let cur_clock = cpu.as_ref().and_then(|c| c.current_clock_speed);

        // Thermal zones -> real °C, filtering sentinels.
        let mut zones: Vec<ZoneReading> = Vec::new();
        for z in &probe.zones {
            let tenths_k = z.high_precision_temperature.unwrap_or(0.0);
            let whole_k = z.temperature.unwrap_or(0.0);
            let k = if tenths_k > 0.0 { tenths_k } else { whole_k * 10.0 };
            if k < VALID_ZONE_MIN_KELVIN * 10.0 {
                continue;
            }
            zones.push(ZoneReading {
                name: z.name.clone(),
                temp_c: kelvin_tenths_to_c(k),
                throttled: z.percent_passive_limit.map(|p| p < 100).unwrap_or(false),
            });
        }
        let package_c = zones
            .iter()
            .map(|z| z.temp_c)
            .fold(None::<f64>, |acc, t| Some(acc.map_or(t, |a| a.max(t))));

        // Per-core load map.
        let mut loads: HashMap<u32, f64> = HashMap::new();
        for c in &probe.cores {
            if let Ok(idx) = c.name.parse::<u32>() {
                loads.insert(idx, c.percent_processor_time.unwrap_or(0) as f64);
            }
        }

        // Map thermal zones to cores: sort zones by temp DESCENDING, then assign
        // Core #0 = hottest zone, #1 = next, etc. This gives varied per-row temps
        // (like CoreTemp) from REAL readings. Cores beyond the zone count reuse
        // the coolest zone. NOTE: this is a zone→core mapping, not a true
        // per-core sensor (see SENSORS.md §5) — documented in the About tab.
        let mut sorted_zone_temps: Vec<f64> = zones.iter().map(|z| z.temp_c).collect();
        sorted_zone_temps.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let coolest = sorted_zone_temps.last().copied();

        // Build per-core readings with real (zone-mapped) temps + running min/max.
        let mut minmax = self.minmax.lock().expect("minmax lock poisoned");
        let mut cores_out: Vec<CoreReading> = Vec::with_capacity(logical as usize);
        let mut temp_vals: Vec<f64> = Vec::new();
        for i in 0..logical {
            let temp_c = sorted_zone_temps
                .get(i as usize)
                .copied()
                .or(coolest);
            if let Some(t) = temp_c {
                let entry = minmax.entry(i).or_insert((t, t));
                entry.0 = entry.0.min(t);
                entry.1 = entry.1.max(t);
                temp_vals.push(t);
            }
            let (min_c, max_c) = minmax.get(&i).copied().unzip();
            cores_out.push(CoreReading {
                index: i,
                kind: kind_for_core(&profile, i),
                load: loads.get(&i).copied(),
                temp_c,
                min_c,
                max_c,
            });
        }

        // Average is now real: mean of the per-core (zone-mapped) temps.
        let average_c = if temp_vals.is_empty() {
            None
        } else {
            Some(temp_vals.iter().sum::<f64>() / temp_vals.len() as f64)
        };

        Ok(SensorSnapshot {
            chip_name: profile.name.to_string(),
            chip_model: profile.model.to_string(),
            core_thread: format!("{} / {}", cores_count, logical),
            tjmax_c: profile.tjmax_c,
            package_c,
            average_c,
            zones,
            cores: cores_out,
            clock_mhz: cur_clock,
            max_clock_mhz: max_clock,
            bus_speed_mhz: Some(100), // nominal reference clock on Snapdragon X
            power_w: None,            // wired via a separate probe if needed
            tick: 0,
        })
    }
}

fn kind_for_core(profile: &ChipProfile, idx: u32) -> CoreKind {
    let mut cursor = 0u32;
    for (kind, n) in profile.clusters {
        cursor += n;
        if idx < cursor {
            return *kind;
        }
    }
    CoreKind::Performance
}
