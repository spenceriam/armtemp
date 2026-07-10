//! Shared sensor data types. These are serialized to the frontend as the
//! `sensor-update` Tauri event payload.
//!
//! Everything is strictly REAL data: any field that has no live source is
//! `Option` and becomes `None` -> the UI renders an honest "—". We never
//! fabricate values. See SENSORS.md for what is proven to work on
//! Snapdragon X (X1P64100) firmware.

use serde::{Deserialize, Serialize};

/// One core's live snapshot. Snapdragon X exposes no per-core thermal
/// surface, so temperature is not tracked per core (see `SensorSnapshot`'s
/// `package_c` for the one real CPU temperature). Load is genuinely
/// per-core and always available via the perf counter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreReading {
    /// Logical core index (0-based).
    pub index: u32,
    /// Core class label from the chip profile: "P" (Prime/Performance) or "E" (Efficiency).
    pub kind: CoreKind,
    /// Display badge for this core's tier, per the chip's real vocabulary —
    /// see `chips::ChipProfile::tier_badge`. Never hardcode "P"/"E" in the
    /// frontend; use this instead (e.g. X1 is all "P", X2 is "P"/"P2").
    pub kind_label: String,
    /// Tooltip text for `kind_label`, e.g. "Prime core" / "Performance core".
    pub kind_title: String,
    /// Utilization 0..=100, always real (perf counter).
    pub load: Option<f64>,
    /// Running minimum load seen since start (real only).
    pub load_min: Option<f64>,
    /// Running maximum load seen since start (real only).
    pub load_max: Option<f64>,
    /// Running session average load since start.
    pub load_avg: Option<f64>,
}

/// Core classification. Snapdragon X Prime/Performance cores map to `Performance`;
/// Efficiency cores to `Efficiency`. Derived from the detected chip profile.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CoreKind {
    Performance,
    Efficiency,
}

/// A distinct thermal zone reading. On Snapdragon X the firmware exposes many
/// ACPI zones (`\_SB.TZxx`) via the perf counter; we surface the valid ones and
/// let the UI/user pick which represents the CPU package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneReading {
    /// ACPI object name, e.g. `\_SB.TZ0`.
    pub name: String,
    /// Temperature in degrees Celsius (real; only present for valid zones).
    pub temp_c: f64,
    /// True if this zone currently reports active passive throttling.
    pub throttled: bool,
}

/// The full snapshot emitted to the frontend each poll tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorSnapshot {
    /// Detected SoC marketing name, e.g. "Snapdragon X Plus".
    pub chip_name: String,
    /// Detected model string, e.g. "X1P64100".
    pub chip_model: String,
    /// Total physical cores / logical threads, e.g. "10 / 10".
    pub core_thread: String,
    /// Platform label, e.g. "ARM64 · Oryon". Built from the detected chip
    /// profile's `uarch`; "ARM64" alone when the microarchitecture is unknown.
    pub platform: String,
    /// Process node from the chip profile, e.g. "4 nm" (spec label, not telemetry).
    pub lithography: String,
    /// Nominal TDP in watts from the chip profile (spec label). `None` when unknown.
    pub tdp_w: Option<u32>,
    /// Thermal junction max in °C from the chip profile (e.g. 100). Used for
    /// the temp-color scale and overheat threshold defaults.
    pub tjmax_c: f64,
    /// CPU temperature (°C), current reading — the hottest valid CPU-area
    /// zone. This IS the app's single honest "CPU temperature"; `None` if no
    /// valid zone exists this tick.
    pub package_c: Option<f64>,
    /// Running minimum of `package_c` since app start (session, real only).
    pub package_min_c: Option<f64>,
    /// Running maximum of `package_c` since app start (session, real only).
    pub package_max_c: Option<f64>,
    /// Running session average of `package_c` since app start.
    pub package_avg_c: Option<f64>,
    /// All valid thermal zones. The UI features the package; the rest are
    /// available for a zones view.
    pub zones: Vec<ZoneReading>,
    /// Per-core readings (load is genuinely per-core and real; there is no
    /// per-core temperature on this firmware — see `package_c`).
    pub cores: Vec<CoreReading>,
    /// Current package clock in MHz (real, live PDH counter).
    pub clock_mhz: Option<u32>,
    /// Base clock in MHz from the chip profile (spec label, matches Task
    /// Manager's "Base speed" — not live telemetry).
    pub base_clock_mhz: Option<u32>,
    /// Boost/max clock in MHz from the chip profile (spec label — not live
    /// telemetry).
    pub max_clock_mhz: Option<u32>,
    /// Nominal bus/reference clock in MHz (100 on Snapdragon X; informational).
    pub bus_speed_mhz: Option<u32>,
    /// Package power in watts. Always `None` — confirmed unavailable from
    /// userspace on this firmware (see SENSORS.md §3).
    pub power_w: Option<f64>,
    /// The registry `Identifier` string, e.g. "ARMv8 (64-bit) Family 8 Model 2
    /// Revision 201" — the machine's real CPUID-derived identity, shown in
    /// the UI's CPUID field instead of repeating the marketing model string.
    pub cpu_identifier: Option<String>,
    /// How `chip_name`/`chip_model` were determined — see `chips::MatchBasis`.
    /// Surfaced so an inferred or unconfirmed SKU is never presented as if it
    /// were read directly off the chip.
    pub detection_basis: String,
    /// Monotonic tick counter so the UI can detect stale updates.
    pub tick: u64,
}

impl Default for SensorSnapshot {
    fn default() -> Self {
        Self {
            chip_name: "Detecting…".into(),
            chip_model: String::new(),
            core_thread: "— / —".into(),
            platform: "ARM64".into(),
            lithography: String::new(),
            tdp_w: None,
            tjmax_c: 100.0,
            package_c: None,
            package_min_c: None,
            package_max_c: None,
            package_avg_c: None,
            zones: Vec::new(),
            cores: Vec::new(),
            clock_mhz: None,
            base_clock_mhz: None,
            max_clock_mhz: None,
            bus_speed_mhz: None,
            power_w: None,
            cpu_identifier: None,
            detection_basis: String::new(),
            tick: 0,
        }
    }
}

/// Decode a raw ACPI/Kelvin temperature into Celsius. On this firmware the perf
/// counter `HighPrecisionTemperature` is in tenths of Kelvin and `Temperature`
/// is in Kelvin; both share the 273.15 offset.
pub fn kelvin_tenths_to_c(tenths_k: f64) -> f64 {
    tenths_k / 10.0 - 273.15
}

/// A zone reading is considered a real live sensor only above this Kelvin
/// threshold. This covers the −40 °C (233 K) and −18 °C (255 K) sentinel
/// placeholders and 0-inactive zones seen on Snapdragon X firmware.
/// See SENSORS.md §1.
pub const VALID_ZONE_MIN_KELVIN: f64 = 273.0;
