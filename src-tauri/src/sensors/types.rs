//! Shared sensor data types. These are serialized to the frontend as the
//! `sensor-update` Tauri event payload.
//!
//! Everything is strictly REAL data: any field that has no live source is
//! `Option` and becomes `None` -> the UI renders an honest "—". We never
//! fabricate values. See SENSORS.md for what is proven to work on
//! Snapdragon X (X1P64100) firmware.

use serde::{Deserialize, Serialize};

/// One core's live snapshot. Temperature may be `None` when the firmware
/// exposes no per-core thermal surface (the common case on Snapdragon X);
/// load is always available via the perf counter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreReading {
    /// Logical core index (0-based).
    pub index: u32,
    /// Core class label from the chip profile: "P" (Prime/Performance) or "E" (Efficiency).
    pub kind: CoreKind,
    /// Utilization 0..=100, always real (perf counter).
    pub load: Option<f64>,
    /// Temperature in degrees Celsius. `None` if no real per-core source.
    pub temp_c: Option<f64>,
    /// Running minimum temp seen since start (real only).
    pub min_c: Option<f64>,
    /// Running maximum temp seen since start (real only).
    pub max_c: Option<f64>,
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
    /// Thermal junction max in °C from the chip profile (e.g. 100). Used for
    /// the temp-color scale and overheat threshold defaults.
    pub tjmax_c: f64,
    /// Package temperature (°C). The hottest valid CPU-area zone; `None` if no
    /// valid zone exists this tick.
    pub package_c: Option<f64>,
    /// Average across the per-core temps that ARE real this tick.
    pub average_c: Option<f64>,
    /// All valid thermal zones. The UI features the package; the rest are
    /// available for a zones view.
    pub zones: Vec<ZoneReading>,
    /// Per-core readings (load always real; temp when available).
    pub cores: Vec<CoreReading>,
    /// Current package clock in MHz (real, from Win32_Processor).
    pub clock_mhz: Option<u32>,
    /// Boost/max clock in MHz (real, from Win32_Processor).
    pub max_clock_mhz: Option<u32>,
    /// Package power in watts (real, from the Power Meter counter when present).
    pub power_w: Option<f64>,
    /// Monotonic tick counter so the UI can detect stale updates.
    pub tick: u64,
}

impl Default for SensorSnapshot {
    fn default() -> Self {
        Self {
            chip_name: "Detecting…".into(),
            chip_model: String::new(),
            core_thread: "— / —".into(),
            tjmax_c: 100.0,
            package_c: None,
            average_c: None,
            zones: Vec::new(),
            cores: Vec::new(),
            clock_mhz: None,
            max_clock_mhz: None,
            power_w: None,
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
