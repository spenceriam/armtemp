//! Native Windows PDH (Performance Data Helper) sensor provider. Replaces the
//! PowerShell shell-out (`powershell.rs`): no per-tick process spawn, and no
//! COM/WMI — PDH is a plain Win32 API (`pdh.dll`) that never touches
//! `IWbemServices`, so it sidesteps the `WBEM_E_NOT_FOUND` failure the `wmi`
//! crate hit when called from inside a Tauri process (see SENSORS.md §7).
//!
//! All data is strictly REAL. Missing/failed counters -> `None`. No fabrication.
//!
//! THREAD MODEL: PDH query/counter handles must be collected from a consistent
//! thread. A single dedicated OS thread owns them for the app's lifetime;
//! `PdhProvider` is just an `mpsc` sender (Clone + Send + Sync) that requests a
//! snapshot/profile and waits for the reply, so it slots into the existing
//! `spawn_blocking`-based poll loop in `lib.rs` unchanged.

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

use windows::core::HSTRING;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhGetFormattedCounterValue, PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::System::SystemInformation::GetSystemInfo;
use windows::core::PCWSTR;

use crate::sensors::chips::{match_profile, ChipProfile};
use crate::sensors::types::{
    kelvin_tenths_to_c, CoreKind, CoreReading, SensorSnapshot, ZoneReading, VALID_ZONE_MIN_KELVIN,
};

const ERROR_SUCCESS: u32 = 0;
const PDH_CSTATUS_VALID_DATA: u32 = 0;
const PDH_CSTATUS_NEW_DATA: u32 = 1;
const PDH_MORE_DATA: u32 = 0x800007D2;

// ---------- public handle ----------

enum Request {
    Snapshot(mpsc::Sender<anyhow::Result<SensorSnapshot>>),
    Profile(mpsc::Sender<Option<ChipProfile>>),
}

/// Handle to the PDH-backed provider. Cheap to clone; all real work happens on
/// the dedicated worker thread spawned in `new()`.
#[derive(Clone)]
pub struct PdhProvider {
    tx: mpsc::Sender<Request>,
}

impl Default for PdhProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PdhProvider {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Request>();
        thread::Builder::new()
            .name("armtemp-pdh".into())
            .spawn(move || worker(rx))
            .expect("failed to spawn PDH worker thread");
        Self { tx }
    }

    /// Detect the chip once (cached on the worker thread) and return its profile.
    pub fn profile(&self) -> anyhow::Result<Option<ChipProfile>> {
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .send(Request::Profile(rtx))
            .map_err(|_| anyhow::anyhow!("pdh worker thread is gone"))?;
        Ok(rrx.recv().unwrap_or(None))
    }

    /// Collect a full real snapshot via the native PDH query.
    pub fn snapshot(&self) -> anyhow::Result<SensorSnapshot> {
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .send(Request::Snapshot(rtx))
            .map_err(|_| anyhow::anyhow!("pdh worker thread is gone"))?;
        rrx.recv()
            .map_err(|_| anyhow::anyhow!("pdh worker thread is gone"))?
    }
}

// ---------- worker thread: owns all PDH handles ----------

/// One open PDH query with the counters ARMTEMP needs. Closed on drop.
struct Query {
    hquery: isize,
    zone_temp: isize,
    zone_hp_temp: isize,
    /// `% Passive Limit` (throttle indicator) — optional; some firmware may
    /// not expose it, in which case `throttled` degrades to `false`.
    zone_passive: Option<isize>,
    core_load: isize,
    /// `\Processor Information(_Total)\Processor Frequency` — a genuinely
    /// live value (unlike `Win32_Processor.CurrentClockSpeed`, which mirrors
    /// the static max on this firmware).
    freq: isize,
}

impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.hquery);
        }
    }
}

fn add_counter(hquery: isize, path: &str) -> anyhow::Result<isize> {
    unsafe {
        let mut hcounter: isize = 0;
        let hpath = HSTRING::from(path);
        let st = PdhAddEnglishCounterW(hquery, &hpath, 0, &mut hcounter);
        if st != ERROR_SUCCESS {
            anyhow::bail!("PdhAddEnglishCounterW({path}) failed: 0x{st:08X}");
        }
        Ok(hcounter)
    }
}

fn open_query() -> anyhow::Result<Query> {
    unsafe {
        let mut hquery: isize = 0;
        let st = PdhOpenQueryW(PCWSTR::null(), 0, &mut hquery);
        if st != ERROR_SUCCESS {
            anyhow::bail!("PdhOpenQueryW failed: 0x{st:08X}");
        }

        // If a required counter fails to add, bail out entirely (retried next
        // tick); the optional passive-limit counter degrades gracefully.
        let result = (|| -> anyhow::Result<Query> {
            let zone_temp = add_counter(hquery, r"\Thermal Zone Information(*)\Temperature")?;
            let zone_hp_temp =
                add_counter(hquery, r"\Thermal Zone Information(*)\High Precision Temperature")?;
            let zone_passive =
                add_counter(hquery, r"\Thermal Zone Information(*)\% Passive Limit").ok();
            let core_load = add_counter(hquery, r"\Processor Information(*)\% Processor Time")?;
            let freq = add_counter(hquery, r"\Processor Information(_Total)\Processor Frequency")?;
            Ok(Query {
                hquery,
                zone_temp,
                zone_hp_temp,
                zone_passive,
                core_load,
                freq,
            })
        })();

        match result {
            Ok(q) => {
                // Prime: rate-style counters (% Processor Time, Processor
                // Frequency) return PDH_INVALID_DATA on the very first
                // collect. This primes the query so the first real tick
                // (which collects again) already has valid rate data.
                let _ = PdhCollectQueryData(q.hquery);
                Ok(q)
            }
            Err(e) => {
                let _ = PdhCloseQuery(hquery);
                Err(e)
            }
        }
    }
}

/// Read a multi-instance counter into `(instance_name, value)` pairs, keeping
/// only instances with valid/new data this tick. Never panics on bad data —
/// an unreadable counter just yields an empty vec.
fn read_array(hcounter: isize) -> Vec<(String, f64)> {
    unsafe {
        let mut buf_size: u32 = 0;
        let mut item_count: u32 = 0;
        let st = PdhGetFormattedCounterArrayW(hcounter, PDH_FMT_DOUBLE, &mut buf_size, &mut item_count, None);
        if st != PDH_MORE_DATA || buf_size == 0 {
            return Vec::new();
        }
        // Allocate in u64 words so pointer-sized fields inside
        // PDH_FMT_COUNTERVALUE_ITEM_W land on a naturally aligned address.
        let words = (buf_size as usize).div_ceil(8);
        let mut buf: Vec<u64> = vec![0u64; words];
        let ptr = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        let st2 =
            PdhGetFormattedCounterArrayW(hcounter, PDH_FMT_DOUBLE, &mut buf_size, &mut item_count, Some(ptr));
        if st2 != ERROR_SUCCESS {
            return Vec::new();
        }
        let items = std::slice::from_raw_parts(ptr, item_count as usize);
        let mut out = Vec::with_capacity(item_count as usize);
        for item in items {
            let status = item.FmtValue.CStatus;
            if status != PDH_CSTATUS_VALID_DATA && status != PDH_CSTATUS_NEW_DATA {
                continue;
            }
            let Ok(name) = item.szName.to_string() else {
                continue;
            };
            out.push((name, item.FmtValue.Anonymous.doubleValue));
        }
        out
    }
}

/// Read a single-instance counter value (e.g. the `_Total` live frequency).
fn read_single(hcounter: isize) -> Option<f64> {
    unsafe {
        let mut val = PDH_FMT_COUNTERVALUE::default();
        let st = PdhGetFormattedCounterValue(hcounter, PDH_FMT_DOUBLE, None, &mut val);
        if st == ERROR_SUCCESS
            && (val.CStatus == PDH_CSTATUS_VALID_DATA || val.CStatus == PDH_CSTATUS_NEW_DATA)
        {
            Some(val.Anonymous.doubleValue)
        } else {
            None
        }
    }
}

/// `Processor Information` per-core instances are named `group,core` (e.g.
/// `0,3`) plus `group,_Total`/bare `_Total` roll-ups — parse the trailing
/// index and skip totals. (Differs from the flat `0`.. `9` index the old
/// `Win32_PerfFormattedData_PerfOS_Processor`/WMI path used.)
fn parse_core_index(instance_name: &str) -> Option<u32> {
    let tail = instance_name.rsplit(',').next()?;
    if tail.eq_ignore_ascii_case("_total") {
        return None;
    }
    tail.trim().parse::<u32>().ok()
}

/// CPU name from the registry — no COM/PowerShell required.
fn read_processor_name() -> Option<String> {
    unsafe {
        let subkey = HSTRING::from(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0");
        let value = HSTRING::from("ProcessorNameString");
        let mut size: u32 = 0;
        let st = RegGetValueW(HKEY_LOCAL_MACHINE, &subkey, &value, RRF_RT_REG_SZ, None, None, Some(&mut size));
        if st.0 != ERROR_SUCCESS || size == 0 {
            return None;
        }
        let mut buf: Vec<u16> = vec![0u16; (size as usize).div_ceil(2)];
        let st2 = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &subkey,
            &value,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            Some(&mut size),
        );
        if st2.0 != ERROR_SUCCESS {
            return None;
        }
        let s = String::from_utf16_lossy(&buf);
        Some(s.trim_end_matches('\0').trim().to_string())
    }
}

/// Logical processor count via `GetSystemInfo` (native, no COM/PowerShell).
/// Oryon has no SMT, so physical == logical on every known Snapdragon X SKU.
fn logical_core_count() -> u32 {
    use windows::Win32::System::SystemInformation::SYSTEM_INFO;
    unsafe {
        let mut info = SYSTEM_INFO::default();
        GetSystemInfo(&mut info);
        info.dwNumberOfProcessors.max(1)
    }
}

fn detect_profile() -> ChipProfile {
    let cores = logical_core_count();
    match read_processor_name() {
        Some(name) => match_profile(&name.to_lowercase(), cores),
        None => match_profile("snapdragon", cores),
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

/// Per-core running stats across ticks: (min, max, sum, sample_count).
type CoreStats = HashMap<u32, (f64, f64, f64, u64)>;

fn build_snapshot(q: &Query, profile: &ChipProfile, stats: &mut CoreStats) -> SensorSnapshot {
    unsafe {
        let _ = PdhCollectQueryData(q.hquery);
    }

    // Merge whole-Kelvin + tenths-of-Kelvin readings by instance name; prefer
    // the high-precision value when present (matches powershell.rs's rule).
    let whole: HashMap<String, f64> = read_array(q.zone_temp).into_iter().collect();
    let precise: HashMap<String, f64> = read_array(q.zone_hp_temp).into_iter().collect();
    let passive: HashMap<String, f64> = q
        .zone_passive
        .map(read_array)
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut zones: Vec<ZoneReading> = Vec::new();
    for (name, whole_k) in &whole {
        let tenths_k = precise.get(name).copied().unwrap_or(0.0);
        let k = if tenths_k > 0.0 { tenths_k } else { whole_k * 10.0 };
        if k < VALID_ZONE_MIN_KELVIN * 10.0 {
            continue;
        }
        let throttled = passive.get(name).map(|p| *p < 100.0).unwrap_or(false);
        zones.push(ZoneReading {
            name: name.clone(),
            temp_c: kelvin_tenths_to_c(k),
            throttled,
        });
    }
    let package_c = zones
        .iter()
        .map(|z| z.temp_c)
        .fold(None::<f64>, |acc, t| Some(acc.map_or(t, |a| a.max(t))));

    // Per-core load, keyed by the parsed core index.
    let mut loads: HashMap<u32, f64> = HashMap::new();
    for (name, value) in read_array(q.core_load) {
        if let Some(idx) = parse_core_index(&name) {
            loads.insert(idx, value);
        }
    }

    let logical = logical_core_count();
    let cores_count = profile.total_cores().max(logical);

    // Zone -> core mapping: hottest zone first (same "core cluster" proxy as
    // the PowerShell backend — see SENSORS.md §5, no true per-core sensor).
    let mut sorted_zone_temps: Vec<f64> = zones.iter().map(|z| z.temp_c).collect();
    sorted_zone_temps.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let coolest = sorted_zone_temps.last().copied();

    let mut cores_out: Vec<CoreReading> = Vec::with_capacity(logical as usize);
    let mut temp_vals: Vec<f64> = Vec::new();
    for i in 0..logical {
        let temp_c = sorted_zone_temps.get(i as usize).copied().or(coolest);
        if let Some(t) = temp_c {
            let entry = stats.entry(i).or_insert((t, t, 0.0, 0));
            entry.0 = entry.0.min(t);
            entry.1 = entry.1.max(t);
            entry.2 += t;
            entry.3 += 1;
            temp_vals.push(t);
        }
        let s = stats.get(&i);
        cores_out.push(CoreReading {
            index: i,
            kind: kind_for_core(profile, i),
            load: loads.get(&i).copied(),
            temp_c,
            min_c: s.map(|s| s.0),
            max_c: s.map(|s| s.1),
            avg_c: s.map(|s| s.2 / s.3 as f64),
        });
    }

    let average_c = if temp_vals.is_empty() {
        None
    } else {
        Some(temp_vals.iter().sum::<f64>() / temp_vals.len() as f64)
    };

    // Live frequency: genuinely varies with load (unlike the static
    // Win32_Processor.CurrentClockSpeed this backend replaces). `max_clock_mhz`
    // comes from the chip profile's boost clock — informational, not live.
    let clock_mhz = read_single(q.freq).map(|v| v.round() as u32);
    let max_clock_mhz = if profile.boost_ghz > 0.0 {
        Some((profile.boost_ghz * 1000.0).round() as u32)
    } else {
        None
    };

    let platform = if profile.uarch.is_empty() {
        "ARM64".to_string()
    } else {
        format!("ARM64 · {}", profile.uarch)
    };

    SensorSnapshot {
        chip_name: profile.name.to_string(),
        chip_model: profile.model.to_string(),
        core_thread: format!("{} / {}", cores_count, logical),
        platform,
        lithography: profile.lithography.to_string(),
        tdp_w: if profile.tdp_w > 0 { Some(profile.tdp_w) } else { None },
        tjmax_c: profile.tjmax_c,
        package_c,
        average_c,
        zones,
        cores: cores_out,
        clock_mhz,
        max_clock_mhz,
        bus_speed_mhz: Some(100), // nominal reference clock on Snapdragon X
        power_w: None,            // confirmed empty from userspace on this firmware
        tick: 0,
    }
}

fn worker(rx: mpsc::Receiver<Request>) {
    let profile = detect_profile();
    let mut stats: CoreStats = HashMap::new();
    let mut query: Option<Query> = None;

    for req in rx {
        match req {
            Request::Profile(reply) => {
                let _ = reply.send(Some(profile.clone()));
            }
            Request::Snapshot(reply) => {
                if query.is_none() {
                    query = open_query().ok();
                }
                let result = match &query {
                    Some(q) => Ok(build_snapshot(q, &profile, &mut stats)),
                    None => Err(anyhow::anyhow!("PDH query not open")),
                };
                if result.is_err() {
                    // Force a reopen attempt next tick instead of spinning on
                    // a broken query.
                    query = None;
                }
                let _ = reply.send(result);
            }
        }
    }
}
