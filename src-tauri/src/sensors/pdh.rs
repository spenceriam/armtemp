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
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GetSystemInfo, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows::core::PCWSTR;

use crate::sensors::chips::{match_profile, ChipProfile, Detection, MatchBasis};
use crate::sensors::identity::{self, CpuIdentity};
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
    DetectionReport(mpsc::Sender<String>),
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

    /// A plain-text dump of every raw identity signal + how the chip was
    /// matched — for the Tools menu's "Copy detection report" (see issue #2:
    /// this is how a reporter can hand over one paste instead of screenshots).
    pub fn detection_report(&self) -> anyhow::Result<String> {
        let (rtx, rrx) = mpsc::channel();
        self.tx
            .send(Request::DetectionReport(rtx))
            .map_err(|_| anyhow::anyhow!("pdh worker thread is gone"))?;
        rrx.recv()
            .map_err(|_| anyhow::anyhow!("pdh worker thread is gone"))
    }
}

// ---------- worker thread: owns all PDH handles ----------

/// One open PDH query with the counters ARMtemp needs. Closed on drop.
struct Query {
    hquery: isize,
    zone_temp: isize,
    zone_hp_temp: isize,
    /// `% Passive Limit` (throttle indicator) — optional; some firmware may
    /// not expose it, in which case `throttled` degrades to `false`.
    zone_passive: Option<isize>,
    core_load: isize,
    /// `\Processor Information(_Total)\Processor Frequency` — fallback Speed
    /// source when the per-core formula below isn't available. On a
    /// heterogeneous P/E chip this is a blended average across clusters (see
    /// issue #2: it under-reports a busy Prime cluster whenever the
    /// Efficiency cluster is idle), so `perf_pct` is preferred whenever it's
    /// present.
    freq: isize,
    /// `% Processor Performance` per core — this tick's utilization relative
    /// to that core's OWN nominal/rated frequency; can exceed 100% under
    /// boost. Combined with `CpuIdentity::per_core_mhz` (that same core's
    /// rated clock) this reproduces Task Manager's documented Speed formula,
    /// correctly per-cluster on heterogeneous chips. Optional — some
    /// firmware may not expose it.
    perf_pct: Option<isize>,
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
        // tick); the optional passive-limit/perf-performance counters
        // degrade gracefully.
        let result = (|| -> anyhow::Result<Query> {
            let zone_temp = add_counter(hquery, r"\Thermal Zone Information(*)\Temperature")?;
            let zone_hp_temp =
                add_counter(hquery, r"\Thermal Zone Information(*)\High Precision Temperature")?;
            let zone_passive =
                add_counter(hquery, r"\Thermal Zone Information(*)\% Passive Limit").ok();
            let core_load = add_counter(hquery, r"\Processor Information(*)\% Processor Time")?;
            let freq = add_counter(hquery, r"\Processor Information(_Total)\Processor Frequency")?;
            let perf_pct = add_counter(hquery, r"\Processor Information(*)\% Processor Performance").ok();
            Ok(Query {
                hquery,
                zone_temp,
                zone_hp_temp,
                zone_passive,
                core_load,
                freq,
                perf_pct,
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

/// Query the OS's real per-core Efficiency Class via
/// `GetLogicalProcessorInformationEx(RelationProcessorCore)`, keyed by
/// logical core index (bit position in the group affinity mask). `None` if
/// the API call fails, the topology spans more than one processor group
/// (not expected — every known Snapdragon X/X2 SKU has far fewer than the
/// 64 logical cores a single group holds), or any index in
/// `0..logical_cores` was left unaccounted for by the returned entries.
fn query_core_efficiency_classes(logical_cores: u32) -> Option<Vec<u8>> {
    unsafe {
        let mut len: u32 = 0;
        // Size probe: expected to return an error while still filling `len`
        // with the required buffer size.
        let _ = GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut len);
        if len == 0 {
            return None;
        }

        let mut buf: Vec<u8> = vec![0u8; len as usize];
        let ptr = buf.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
        GetLogicalProcessorInformationEx(RelationProcessorCore, Some(ptr), &mut len).ok()?;

        // The buffer is a run of variable-length entries; `Size` on each one
        // is the only reliable way to find the next entry's offset.
        let mut classes = vec![0u8; logical_cores as usize];
        let mut seen = vec![false; logical_cores as usize];
        let mut offset = 0usize;
        while offset < buf.len() {
            let entry =
                &*(buf.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX);
            let entry_size = entry.Size as usize;
            if entry_size == 0 {
                break; // malformed; stop rather than looping forever
            }
            let proc = &entry.Anonymous.Processor;
            if proc.GroupCount == 1 {
                let mask = proc.GroupMask[0].Mask;
                for bit in 0..(logical_cores as usize).min(usize::BITS as usize) {
                    if (mask >> bit) & 1 == 1 {
                        classes[bit] = proc.EfficiencyClass;
                        seen[bit] = true;
                    }
                }
            }
            offset += entry_size;
        }

        if seen.iter().all(|s| *s) {
            Some(classes)
        } else {
            None
        }
    }
}

/// Maps each core's raw EfficiencyClass byte (in core-index order) to a P/E
/// `CoreKind`: cores at the highest class present become `Performance`,
/// every lower class becomes `Efficiency`. A single distinct class
/// (homogeneous chip — X1 Elite, X2P-42) maps everything to `Performance`.
fn classes_to_kinds(classes: &[u8]) -> Vec<CoreKind> {
    let max = classes.iter().copied().max().unwrap_or(0);
    classes
        .iter()
        .map(|&c| {
            if c == max {
                CoreKind::Performance
            } else {
                CoreKind::Efficiency
            }
        })
        .collect()
}

/// Real per-core P/E classification straight from the OS topology API.
/// `None` falls back to the positional walk over the static profile's
/// `clusters` table in `kind_for_core` below — see issue #2: a screenshot
/// relayed on that issue suggested X2 enumerates Efficiency cores before
/// Performance ones, the opposite of every X1 profile's static table, but
/// that evidence is secondhand and not solid enough to hardcode. Querying
/// the live topology sidesteps needing to guess the enumeration order for
/// any given chip at all.
fn real_core_kinds(logical_cores: u32) -> Option<Vec<CoreKind>> {
    query_core_efficiency_classes(logical_cores).map(|classes| classes_to_kinds(&classes))
}

fn kind_for_core(real_kinds: &Option<Vec<CoreKind>>, profile: &ChipProfile, idx: u32) -> CoreKind {
    if let Some(kinds) = real_kinds {
        if let Some(k) = kinds.get(idx as usize) {
            return *k;
        }
    }
    let mut cursor = 0u32;
    for (kind, n) in profile.clusters {
        cursor += n;
        if idx < cursor {
            return *kind;
        }
    }
    CoreKind::Performance
}

/// Real per-cluster Speed: each core's `% Processor Performance` (this
/// tick's utilization vs that core's OWN nominal frequency) times that
/// core's registry `~MHz` (its cluster's rated frequency) — Task Manager's
/// documented formula. Returns the fastest core's effective clock right now.
/// Correct on heterogeneous P/E chips, unlike the single `_Total` average
/// (see issue #2, where an idle Efficiency cluster dragged that average well
/// below the Prime cluster's real running clock). `None` if either input is
/// unavailable, so the caller can fall back to `_Total Processor Frequency`.
fn effective_clock_mhz(perf_pct_by_core: &HashMap<u32, f64>, per_core_mhz: &[u32]) -> Option<u32> {
    if per_core_mhz.is_empty() || perf_pct_by_core.is_empty() {
        return None;
    }
    perf_pct_by_core
        .iter()
        .filter_map(|(idx, pct)| {
            per_core_mhz
                .get(*idx as usize)
                .map(|nominal| (*pct / 100.0) * (*nominal as f64))
        })
        .fold(None::<f64>, |acc, v| Some(acc.map_or(v, |a| a.max(v))))
        .map(|v| v.round() as u32)
}

/// Per-core running LOAD stats across ticks: (min, max, sum, sample_count).
/// Load is genuinely per-core (unlike temperature — see `TempStats` below).
type CoreStats = HashMap<u32, (f64, f64, f64, u64)>;

/// Running session stats for the single CPU temperature (`package_c`):
/// (min, max, sum, sample_count). `None` until the first valid reading.
type TempStats = Option<(f64, f64, f64, u64)>;

fn build_snapshot(
    q: &Query,
    profile: &ChipProfile,
    basis: MatchBasis,
    identity: &CpuIdentity,
    real_kinds: &Option<Vec<CoreKind>>,
    stats: &mut CoreStats,
    temp_stats: &mut TempStats,
) -> SensorSnapshot {
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
    // The single honest CPU temperature: the hottest valid zone. There is no
    // true per-core temperature sensor on this firmware (see SENSORS.md §5) —
    // per-core rows carry LOAD instead, which genuinely is per-core.
    let package_c = zones
        .iter()
        .map(|z| z.temp_c)
        .fold(None::<f64>, |acc, t| Some(acc.map_or(t, |a| a.max(t))));

    // Session running stats (min/max/avg) for that single CPU temperature.
    if let Some(t) = package_c {
        let entry = temp_stats.get_or_insert((t, t, 0.0, 0));
        entry.0 = entry.0.min(t);
        entry.1 = entry.1.max(t);
        entry.2 += t;
        entry.3 += 1;
    }
    let (package_min_c, package_max_c, package_avg_c) = match temp_stats {
        Some(s) => (Some(s.0), Some(s.1), Some(s.2 / s.3 as f64)),
        None => (None, None, None),
    };

    // Per-core load, keyed by the parsed core index. This IS genuinely
    // per-core (unlike temperature).
    let mut loads: HashMap<u32, f64> = HashMap::new();
    for (name, value) in read_array(q.core_load) {
        if let Some(idx) = parse_core_index(&name) {
            loads.insert(idx, value);
        }
    }

    let logical = logical_core_count();
    let cores_count = profile.total_cores().max(logical);

    let mut cores_out: Vec<CoreReading> = Vec::with_capacity(logical as usize);
    for i in 0..logical {
        let load = loads.get(&i).copied();
        if let Some(l) = load {
            let entry = stats.entry(i).or_insert((l, l, 0.0, 0));
            entry.0 = entry.0.min(l);
            entry.1 = entry.1.max(l);
            entry.2 += l;
            entry.3 += 1;
        }
        let s = stats.get(&i);
        cores_out.push(CoreReading {
            index: i,
            kind: kind_for_core(real_kinds, profile, i),
            load,
            load_min: s.map(|s| s.0),
            load_max: s.map(|s| s.1),
            load_avg: s.map(|s| s.2 / s.3 as f64),
        });
    }

    // Live frequency: per-core `% Processor Performance` × that core's real
    // rated `~MHz`, correct on heterogeneous P/E chips (see
    // `effective_clock_mhz`); falls back to the `_Total` blended-average
    // counter only if that per-core data isn't available this tick.
    let mut perf_pct_by_core: HashMap<u32, f64> = HashMap::new();
    if let Some(perf_pct) = q.perf_pct {
        for (name, value) in read_array(perf_pct) {
            if let Some(idx) = parse_core_index(&name) {
                perf_pct_by_core.insert(idx, value);
            }
        }
    }
    let clock_mhz = effective_clock_mhz(&perf_pct_by_core, &identity.per_core_mhz)
        .or_else(|| read_single(q.freq).map(|v| v.round() as u32));
    let base_clock_mhz = if profile.base_ghz > 0.0 {
        Some((profile.base_ghz * 1000.0).round() as u32)
    } else {
        None
    };
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
        package_min_c,
        package_max_c,
        package_avg_c,
        zones,
        cores: cores_out,
        clock_mhz,
        base_clock_mhz,
        max_clock_mhz,
        bus_speed_mhz: Some(100), // nominal reference clock on Snapdragon X
        power_w: None,            // confirmed empty from userspace on this firmware
        cpu_identifier: identity.identifier.clone(),
        detection_basis: basis.label().to_string(),
        tick: 0,
    }
}

/// Plain-text diagnostics dump: every raw identity signal plus how the chip
/// was matched. Lets a reporter paste one block instead of screenshots.
fn format_detection_report(identity: &CpuIdentity, profile: &ChipProfile, basis: MatchBasis) -> String {
    format!(
        "ARMtemp detection report\n\
         ProcessorNameString: {}\n\
         Identifier: {}\n\
         VendorIdentifier: {}\n\
         Logical cores: {}\n\
         Per-core ~MHz: {:?}\n\
         Real P/E topology: {} performance / {} efficiency\n\
         --- Matched profile ---\n\
         Name: {}\n\
         Model: {}\n\
         Cores: {} ({:?})\n\
         Base/Boost: {:.2} / {:.2} GHz\n\
         Match basis: {}\n",
        identity.name.as_deref().unwrap_or("(none)"),
        identity.identifier.as_deref().unwrap_or("(none)"),
        identity.vendor.as_deref().unwrap_or("(none)"),
        identity.logical_cores,
        identity.per_core_mhz,
        identity.perf_cores.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
        identity.eff_cores.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
        profile.name,
        profile.model,
        profile.total_cores(),
        profile.clusters,
        profile.base_ghz,
        profile.boost_ghz,
        basis.label(),
    )
}

fn worker(rx: mpsc::Receiver<Request>) {
    let logical = logical_core_count();
    let real_kinds = real_core_kinds(logical);
    let (perf_cores, eff_cores) = match &real_kinds {
        Some(kinds) => (
            Some(kinds.iter().filter(|k| **k == CoreKind::Performance).count() as u32),
            Some(kinds.iter().filter(|k| **k == CoreKind::Efficiency).count() as u32),
        ),
        None => (None, None),
    };
    let cpu_identity = identity::gather(logical, perf_cores, eff_cores);
    let Detection { profile, basis } = match_profile(&cpu_identity);
    let mut stats: CoreStats = HashMap::new();
    let mut temp_stats: TempStats = None;
    let mut query: Option<Query> = None;

    for req in rx {
        match req {
            Request::Profile(reply) => {
                let _ = reply.send(Some(profile.clone()));
            }
            Request::DetectionReport(reply) => {
                let _ = reply.send(format_detection_report(&cpu_identity, &profile, basis));
            }
            Request::Snapshot(reply) => {
                if query.is_none() {
                    query = open_query().ok();
                }
                let result = match &query {
                    Some(q) => Ok(build_snapshot(
                        q,
                        &profile,
                        basis,
                        &cpu_identity,
                        &real_kinds,
                        &mut stats,
                        &mut temp_stats,
                    )),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_to_kinds_two_tier_highest_class_is_performance() {
        // X2E-78-100 real topology per issue #2: 6 x Oryon P1 (efficiency
        // class 0) + 6 x Oryon P2 (efficiency class 1) — order in the raw
        // class list doesn't matter, only which class is highest.
        let classes = [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1];
        let kinds = classes_to_kinds(&classes);
        assert_eq!(kinds[..6], [CoreKind::Efficiency; 6]);
        assert_eq!(kinds[6..], [CoreKind::Performance; 6]);
    }

    #[test]
    fn classes_to_kinds_homogeneous_is_all_performance() {
        // X1 Elite / X2P-42: a single Efficiency Class for every core.
        let classes = [0u8; 8];
        let kinds = classes_to_kinds(&classes);
        assert!(kinds.iter().all(|k| *k == CoreKind::Performance));
    }

    #[test]
    fn classes_to_kinds_three_tier_only_top_class_is_performance() {
        // Hypothetical 3-tier topology: everything below the max class is
        // Efficiency, not just the immediately-lower tier.
        let classes = [0, 1, 2];
        let kinds = classes_to_kinds(&classes);
        assert_eq!(kinds, [CoreKind::Efficiency, CoreKind::Efficiency, CoreKind::Performance]);
    }

    #[test]
    fn effective_clock_uses_the_fastest_core_not_a_blended_average() {
        // Prime cluster busy at its full rated 4032 MHz; Efficiency cluster
        // idle. The old `_Total` counter would blend these into ~3.7 GHz
        // (issue #2's reported wrong Speed); the per-core formula must
        // report the Prime cluster's real 4032 MHz instead.
        let mut perf_pct = HashMap::new();
        perf_pct.insert(0u32, 100.0); // Prime core at 100% of its own nominal
        perf_pct.insert(6u32, 10.0); // Efficiency core mostly idle
        let per_core_mhz = vec![4032, 4032, 4032, 4032, 4032, 4032, 3400, 3400, 3400, 3400, 3400, 3400];
        assert_eq!(effective_clock_mhz(&perf_pct, &per_core_mhz), Some(4032));
    }

    #[test]
    fn effective_clock_none_when_inputs_unavailable() {
        assert_eq!(effective_clock_mhz(&HashMap::new(), &[]), None);
        let mut perf_pct = HashMap::new();
        perf_pct.insert(0u32, 100.0);
        assert_eq!(effective_clock_mhz(&perf_pct, &[]), None);
    }
}
