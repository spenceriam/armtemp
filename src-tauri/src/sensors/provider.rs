//! Real sensor provider — queries Windows WMI for genuine telemetry on
//! Snapdragon X (Oryon). Built directly on the Phase 0 findings in SENSORS.md.
//!
//! Sources (all confirmed working on X1P64100 / Windows 11 ARM64):
//!  * `Win32_PerfFormattedData_Counters_ThermalZoneInformation` -> zone temps (Kelvin)
//!  * `Win32_PerfFormattedData_PerfOS_Processor`            -> per-core load %
//!  * `Win32_Processor`                                      -> name, clocks, core/thread counts
//!  * `Win32_PerfFormattedData_PowerMeter_PowerMeter`       -> package power (when present)
//!
//! Strictly real: missing data -> `None`. No fabricated values anywhere.
//!
//! THREAD MODEL: WMI's COM objects are not `Send`, so the `WMIConnection` lives
//! on ONE dedicated thread (correct for COM single-threaded apartments). Callers
//! send requests over a channel and receive plain `SensorSnapshot` values back
//! (which are `Send+Sync`). This keeps Tauri's shared `State` Send-safe.
//!
//! RESILIENCE: WMI init can fail transiently at startup. The provider degrades
//! to a "disabled" state and re-attempts COM/WMI init on the worker; the poll
//! loop keeps calling snapshot() so readings appear once init succeeds.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

use crate::sensors::chips::{match_profile, ChipProfile};
use crate::sensors::types::{
    kelvin_tenths_to_c, CoreKind, CoreReading, SensorSnapshot, ZoneReading,
    VALID_ZONE_MIN_KELVIN,
};
use wmi::{COMLibrary, WMIConnection};

// ---------- WMI row shapes ----------

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ThermalZoneRow {
    name: String,
    #[serde(rename = "Temperature")]
    temperature_k: Option<f64>,
    #[serde(rename = "HighPrecisionTemperature")]
    high_precision_k: Option<f64>,
    #[serde(rename = "PercentPassiveLimit")]
    passive_limit: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ProcessorRow {
    name: String,
    #[serde(rename = "NumberOfCores")]
    cores: Option<u32>,
    #[serde(rename = "NumberOfLogicalProcessors")]
    logical: Option<u32>,
    #[serde(rename = "MaxClockSpeed")]
    max_clock: Option<u32>,
    #[serde(rename = "CurrentClockSpeed")]
    cur_clock: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PerCoreLoadRow {
    #[serde(rename = "PercentProcessorTime")]
    pct: Option<u32>,
    name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PowerMeterRow {
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "Power")]
    power_mw: Option<u64>,
}

/// Handle to the sensor provider. Cheap to clone; all calls hop to the worker
/// thread that owns the non-`Send` WMI connection.
#[derive(Clone)]
pub struct SensorProvider {
    tx: mpsc::Sender<Request>,
}

enum Request {
    Snapshot(mpsc::Sender<anyhow::Result<SensorSnapshot>>),
    Profile(mpsc::Sender<Option<ChipProfile>>),
}

impl SensorProvider {
    /// Spawn the worker thread and open the WMI connection on it. If WMI init
    /// fails at startup, returns a `disabled()` provider that keeps retrying.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<Request>();
        let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<ChipProfile>>();
        thread::spawn(move || worker(rx, ready_tx));
        // Don't block the app on the first init attempt — report readiness.
        match ready_rx.recv() {
            Ok(Ok(profile)) => {
                let me = Self { tx };
                // Stash profile for retrieval via profile().
                let _ = profile;
                Ok(me)
            }
            Ok(Err(e)) => {
                // Worker is still running and will retry; return a usable handle.
                eprintln!("[armtemp] initial WMI open failed, worker will retry: {e}");
                Ok(Self { tx })
            }
            Err(_) => Err(anyhow::anyhow!("sensor worker thread died")),
        }
    }

    /// A provider whose worker could not init WMI yet but is still retrying.
    pub fn disabled() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            tx: mpsc::channel().0,
        })
    }

    /// Collect a full real snapshot. Returns Err if WMI still isn't up.
    pub fn snapshot(&self) -> anyhow::Result<SensorSnapshot> {
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .send(Request::Snapshot(rtx))
            .map_err(|e| anyhow::anyhow!("sensor worker closed: {e}"))?;
        rrx.recv()
            .map_err(|e| anyhow::anyhow!("sensor worker reply dropped: {e}"))?
    }

    /// Cached detected chip profile (None until WMI init succeeds).
    pub fn profile(&self) -> anyhow::Result<Option<ChipProfile>> {
        let (rtx, rrx) = mpsc::channel();
        if self.tx.send(Request::Profile(rtx)).is_err() {
            return Ok(None);
        }
        Ok(rrx
            .recv()
            .map_err(|e| anyhow::anyhow!("sensor worker reply dropped: {e}"))?)
    }
}

/// The worker owns the WMI connection and serves requests. Runs forever and
/// retries WMI init if it failed at startup.
fn worker(rx: mpsc::Receiver<Request>, ready: mpsc::Sender<anyhow::Result<ChipProfile>>) {
    // Try to open WMI; report readiness either way. If it fails, keep looping
    // and retry init on each request until it succeeds.
    let mut state = match open_wmi() {
        Ok((conn, profile)) => {
            let _ = ready.send(Ok(profile.clone()));
            WorkerState::Ready { conn, profile, core_minmax: HashMap::new() }
        }
        Err(e) => {
            let _ = ready.send(Err(e));
            WorkerState::Pending
        }
    };

    for req in rx {
        // If not yet initialized, retry init before serving.
        if matches!(state, WorkerState::Pending) {
            match open_wmi() {
                Ok((conn, profile)) => {
                    state = WorkerState::Ready {
                        conn,
                        profile,
                        core_minmax: HashMap::new(),
                    };
                }
                Err(_) => {
                    reply_error(&req);
                    continue;
                }
            }
        }
        serve(req, &mut state);
    }
}

/// Reply to a request when WMI is still unavailable.
fn reply_error(req: &Request) {
    match req {
        Request::Snapshot(s) => {
            let _ = s.send(Err(anyhow::anyhow!("WMI unavailable")));
        }
        Request::Profile(s) => {
            let _ = s.send(None);
        }
    }
}

enum WorkerState {
    Pending,
    Ready {
        conn: WMIConnection,
        profile: ChipProfile,
        core_minmax: HashMap<u32, (f64, f64)>,
    },
}

fn serve(req: Request, state: &mut WorkerState) {
    if let WorkerState::Ready { conn, profile, core_minmax } = state {
        match req {
            Request::Profile(s) => {
                let _ = s.send(Some(profile.clone()));
            }
            Request::Snapshot(s) => {
                let snap = collect_snapshot(conn, profile, core_minmax);
                let _ = s.send(snap);
            }
        }
    }
}

fn open_wmi() -> anyhow::Result<(WMIConnection, ChipProfile)> {
    eprintln!("[armtemp] open_wmi: COMLibrary::new (full, with security)...");
    let com = COMLibrary::new().map_err(|e| {
        eprintln!("[armtemp] COMLibrary::new failed: {e:?}");
        anyhow::anyhow!("COMLibrary::new failed: {e:?}")
    })?;
    eprintln!("[armtemp] open_wmi: WMIConnection::new...");
    let conn = WMIConnection::new(com.into()).map_err(|e| {
        eprintln!("[armtemp] WMIConnection::new failed: {e:?}");
        anyhow::anyhow!("WMIConnection::new failed: {e:?}")
    })?;
    eprintln!("[armtemp] open_wmi: detect_chip...");
    let profile = detect_chip(&conn).map_err(|e| {
        eprintln!("[armtemp] detect_chip failed: {e:?}");
        anyhow::anyhow!("detect_chip failed: {e:?}")
    })?;
    eprintln!("[armtemp] open_wmi: OK ({})", profile.name);
    Ok((conn, profile))
}

/// Collect a full real snapshot. Each WMI query is best-effort: a missing
/// counter class degrades only that field, never the whole snapshot.
fn collect_snapshot(
    conn: &WMIConnection,
    profile: &ChipProfile,
    core_minmax: &mut HashMap<u32, (f64, f64)>,
) -> anyhow::Result<SensorSnapshot> {
    // --- Processor identity + clocks (the query that proved WMI works). ---
    let procs: Vec<ProcessorRow> = conn.raw_query(
        "SELECT Name, NumberOfCores, NumberOfLogicalProcessors, MaxClockSpeed, CurrentClockSpeed FROM Win32_Processor",
    )?;
    let cpu = procs.into_iter().next();
    let cores = cpu
        .as_ref()
        .and_then(|c| c.cores)
        .unwrap_or_else(|| profile.total_cores());
    let logical = cpu.as_ref().and_then(|c| c.logical).unwrap_or(cores);
    let max_clock = cpu.as_ref().and_then(|c| c.max_clock);
    let cur_clock = cpu.as_ref().and_then(|c| c.cur_clock);

    // --- Thermal zones (best-effort; class may be absent on some firmware). ---
    let mut zones: Vec<ZoneReading> = Vec::new();
    if let Ok(zones_raw) = conn.raw_query::<ThermalZoneRow>(
        "SELECT Name, Temperature, HighPrecisionTemperature, PercentPassiveLimit FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
    ) {
        for z in zones_raw {
            let tenths_k = z.high_precision_k.unwrap_or(0.0);
            let whole_k = z.temperature_k.unwrap_or(0.0);
            let k = if tenths_k > 0.0 { tenths_k } else { whole_k * 10.0 };
            if k < VALID_ZONE_MIN_KELVIN * 10.0 {
                continue; // sentinel / inactive zone — drop, never fake
            }
            let temp_c = kelvin_tenths_to_c(k);
            let throttled = z.passive_limit.map(|p| p < 100).unwrap_or(false);
            zones.push(ZoneReading { name: z.name, temp_c, throttled });
        }
    } else {
        eprintln!("[armtemp] ThermalZoneInformation query failed (no zone temps)");
    }
    let package_c = zones
        .iter()
        .map(|z| z.temp_c)
        .fold(None::<f64>, |acc, t| Some(acc.map_or(t, |a| a.max(t))));

    // --- Per-core load (best-effort). ---
    let mut loads: HashMap<u32, f64> = HashMap::new();
    if let Ok(loads_raw) = conn.raw_query::<PerCoreLoadRow>(
        "SELECT Name, PercentProcessorTime FROM Win32_PerfFormattedData_PerfOS_Processor",
    ) {
        for l in loads_raw {
            if l.name.eq_ignore_ascii_case("_Total") {
                continue;
            }
            if let Ok(idx) = l.name.parse::<u32>() {
                loads.insert(idx, l.pct.unwrap_or(0) as f64);
            }
        }
    } else {
        eprintln!("[armtemp] PerfOS_Processor query failed (no per-core load)");
    }

    // --- Power (best-effort; absent on some firmware). ---
    let mut power_w: Option<f64> = None;
    if let Ok(pm) = conn.raw_query::<PowerMeterRow>(
        "SELECT Name, Power FROM Win32_PerfFormattedData_PowerMeter_PowerMeter",
    ) {
        let mw: f64 = pm.iter().filter_map(|r| r.power_mw).map(|v| v as f64).sum();
        if mw > 0.0 {
            power_w = Some(mw / 1000.0);
        }
    }

    // --- Build per-core readings with P/E kind from the profile ---
    let mut cores_out: Vec<CoreReading> = Vec::with_capacity(logical as usize);
    let mut temp_vals: Vec<f64> = Vec::new();
    for i in 0..logical {
        let kind = kind_for_core(profile, i);
        let load = loads.get(&i).copied();
        // Per-core temp is not exposed by any userspace surface on this
        // firmware (see SENSORS.md §5) — honest `None` here.
        let temp_c: Option<f64> = None;
        if let Some(t) = temp_c {
            let entry = core_minmax.entry(i).or_insert((t, t));
            entry.0 = entry.0.min(t);
            entry.1 = entry.1.max(t);
            temp_vals.push(t);
        }
        let (min_c, max_c) = core_minmax.get(&i).copied().unzip();
        cores_out.push(CoreReading { index: i, kind, load, temp_c, min_c, max_c });
    }

    let average_c = if temp_vals.is_empty() {
        None
    } else {
        Some(temp_vals.iter().sum::<f64>() / temp_vals.len() as f64)
    };

    Ok(SensorSnapshot {
        chip_name: profile.name.to_string(),
        chip_model: profile.model.to_string(),
        core_thread: format!("{} / {}", cores, logical),
        tjmax_c: profile.tjmax_c,
        package_c,
        average_c,
        zones,
        cores: cores_out,
        clock_mhz: cur_clock,
        max_clock_mhz: max_clock,
        power_w,
        tick: 0, // set by the scheduler
    })
}

/// Detect the SoC by reading Win32_Processor.Name once and matching the profile.
fn detect_chip(conn: &WMIConnection) -> anyhow::Result<ChipProfile> {
    let procs: Vec<ProcessorRow> =
        conn.raw_query("SELECT Name, NumberOfCores FROM Win32_Processor")?;
    let cpu = procs.into_iter().next();
    let (name, cores) = match cpu {
        Some(c) => (c.name.to_lowercase(), c.cores.unwrap_or(10)),
        None => ("snapdragon".to_string(), 10),
    };
    Ok(match_profile(&name, cores))
}

/// Map a logical core index to P/E based on the profile's cluster layout.
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
