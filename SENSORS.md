# ARMtemp — Sensor Discovery Report (Phase 0)

Probed on: **Snapdragon X 10-core X1P64100 @ 3.40 GHz** (Snapdragon X Plus),
**Windows 11 Pro Build 26200, ARM 64-bit**. Machine is a Surface-class device
(`MSHW` ACPI IDs, `SurfaceSmfClient` / `SurfaceThermalPolicy` drivers).

Scripts: `tools/probe-sensors.ps1`, `tools/probe-deep.ps1`, `tools/probe-thermal.ps1`.

---

## TL;DR — what works for real telemetry

| Data | Source (confirmed working) | Status |
|------|----------------------------|--------|
| **CPU zone temps** (multiple) | `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (perf counter) | ✅ REAL, multiple zones |
| **Per-core load (%)** | `Win32_PerfFormattedData_PerfOS_Processor` | ✅ REAL, per-core |
| **CPU clocks (max/cur)** | `Win32_Processor` (MaxClockSpeed/CurrentClockSpeed) | ✅ REAL (package-level) |
| **Power** | `SurfacePowerMeterDriver` + "Power Meter MAX34417" (`Power` perf counter) | ✅ REAL (needs wiring) |
| **Per-core temperature** | Not exposed by any userspace/WMI surface found | ⚠️ Requires Surface/Qualcomm SMF driver IOCTL (see below) |

**Bottom line:** Real package / multi-zone temperature, real per-core load, and real
power are all achievable **without a custom kernel driver**. The app can ship with
genuinely live data. True *per-core* temperature is not in any queryable surface on
this firmware and would require talking to the `SurfaceSmfClient` /
`SurfaceIhvCpuSmfClient` drivers via private IOCTL (possible future workstream) — but
the multiple CPU-area thermal zones give a faithful "core cluster" picture.

---

## 1. WIN: Thermal Zone Information perf counter

Class: **`Win32_PerfFormattedData_Counters_ThermalZoneInformation`** (and the raw
`..._RawData_...` variant). Returns **36 zones** named after ACPI objects (`\_SB.TZxx`).

Key fields:
- `Temperature` — on this platform reported **in Kelvin** (raw ACPI tenths-Kelvin,
  pre-divided). Convert: **°C = Temperature − 273.15**.
- `HighPrecisionTemperature` — in **tenths of Kelvin** (one more decimal of precision).
  Convert: **°C = HighPrecisionTemperature / 10 − 273.15**. Prefer this when non-zero.
- `PercentPassiveLimit` / `ThrottleReasons` — throttling telemetry (useful for the
  overheat-protection feature and an honest "throttled" indicator).

### Sample readings (machine warm under load)
| Zone | Temperature (K) | → °C | Note |
|------|-----------------|------|------|
| `\_SB.TZ0` | 349 | **75.9** | CPU area ✅ |
| `\_SB.TZ1` | 349 | **75.9** | CPU area ✅ |
| `\_SB.TZ99` | 349 | **75.9** | CPU area ✅ |
| `\_SB.TZ5`, `TZ4` | 348 | 74.9 | CPU area ✅ |
| `\_SB.TZ3`, `TZ2` | 347 | 73.9 | CPU area ✅ |
| `\_SB.TZ6`, `TZ11` | 342 | 68.9 | warm zone ✅ |
| `\_SB.TZ02` | 332 | 58.9 | cooler zone ✅ |
| `\_SB.TZ33`–`TZ37` | 233 | −40.1 | **sentinel** (no sensor) — filter out |
| `\_SB.TZ31`, `TZ32` | 255 | −18.1 | **sentinel** — filter out |
| `\_SB.TZ17`, `TZ04`, `TZ15`… | 0 | — | **inactive zone** — filter out |

### Filtering rule (real-only, no fabrication)
Treat a zone as a valid live reading only when:
**Temperature > 250** (Kelvin), i.e. **°C > −23**. Anything ≤ 250 K (covering the
−40 sentinel, the −18 sentinel, and the 0 inactive zones) is dropped — never shown,
never faked. This is the "real data only" contract.

### Identifying which zones are CPU zones
ACPI object names (`TZ0`…) aren't self-describing. Strategy for v1: surface all
**valid** zones (those passing the filter) labelled by their ACPI name, let the user
pick which to feature as "Package"/"CPU", and ship sensible defaults discovered at
runtime (highest valid zone = package proxy). The mapping can be refined per-OEM in a
later config file.

---

## 2. Per-core load — confirmed real

Class: **`Win32_PerfFormattedData_PerfOS_Processor`** (exclude `Name = '_Total'`).
`PercentProcessorTime` per core, live. Observed cores 5–9 at 82–100 % under load. ✅

Frequency per-core is **not** populated in this perf class on ARM64; only package
`MaxClockSpeed`/`CurrentClockSpeed` from `Win32_Processor` is available.

---

## 3. Power — hardware present, but NOT readable from userspace

Devices/drivers:
- PnP: **"Power Meter MAX34417"** (`ACPI\MAX34417\3`)
- Driver service: **`SurfacePowerMeterDriver`** (`SurfacePowerMeterDriver.sys`)
- Also `SurfacePowerTrackerCore.sys`.

**Correction (post-Phase 0):** the hardware and driver exist, but the corresponding
`Power Meter` perf-counter set (`Win32_PerfFormattedData_PowerMeter_*` / the native PDH
`Power Meter` object) returns **no instances** on this firmware — confirmed empty via
both `Get-CimInstance` and a native PDH query. Package power is therefore genuinely
unavailable from any userspace surface found so far; ARMtemp shows `power_w: None` → "—"
honestly rather than wiring up a value that doesn't exist. Reading it would require the
same driver-IOCTL route as per-core temperature (§5).

---

## 4. What does NOT work (do not use)

- **`MSAcpi_ThermalZoneTemperature` (root/wmi): returns ZERO instances.** This is the
  classic "how to read temp" answer online; it is empty on this Snapdragon X firmware.
- **`Win32_TemperatureProbe`: zero instances.** (Expected.)
- **`root/LibreHardwareMonitor`, `root/OpenHardwareMonitor`: absent** (those tools are
  not installed and, per research, have no native Oryon/ARM64 support anyway).
- **`KernelThermalConstraintChange` / `KernelThermalPolicyChange` (root/wmi): no instances.**
- **1-wire temp sensor driver: no ARM64 support** (industry-known).

---

## 5. Per-core temperature path (future / stretch)

No userspace/WMI surface exposes **per-core** Oryon temperatures here. The realistic
paths, in increasing effort:
1. **Driver IOCTL to `SurfaceSmfClient` / `SurfaceIhvCpuSmfClient`** — these are the
   drivers backing the "Surface Thermal Zone Sensor Driver" (MSHW0188) and CPU SMF
   clients. A usermode client could issue private IOCTLs; feasibility TBD (undocumented).
2. **Qualcomm subsystem thermal manager** (`qcSubsysThermalMgr.sys`) + the many
   "Qualcomm Temperature Sensor Device" / "Qualcomm ADC Temperature Monitor Device"
   PnP entities (QCOM0C58/59/5E/5F/60/61/62/63/64…).
3. **Signed kernel driver** reading Oryon tsens registers directly (most effort;
   driver-signing required).

For v1 we present **multiple real CPU-area thermal zones** (which is genuinely
informative and better than a single package number) plus real per-core load. The UI's
per-core temperature rows will be driven by the closest real zone until the
per-core driver path lands.

---

## 6. Backend implementation plan (as actually shipped)

ARMtemp's shipped backend (`src-tauri/src/sensors/pdh.rs`) reads these same counters
**natively via the Windows PDH API** (`pdh.dll`), not via `Get-CimInstance`/WMI — PDH is
a plain Win32 API that never touches COM, so it also avoids the `WBEM_E_NOT_FOUND`
failure described in §7.

- **`Thermal Zone Information`** (PDH object) — `Temperature` + `High Precision
  Temperature` every ~1.5–2 s, filtered to valid zones (T > 273 K, i.e. °C > 0 —
  `VALID_ZONE_MIN_KELVIN` in `types.rs`), converted to °C. Highest valid zone = the
  single CPU temperature ARMtemp displays (there is no per-core sensor to report
  individually).
- **`Processor Information`** — `% Processor Time` per core (instances named
  `group,core`, e.g. `0,3`) and `Processor Frequency` on `_Total` — a genuinely **live**
  frequency, unlike `Win32_Processor.CurrentClockSpeed` (§1 note below).
- **Registry + `GetSystemInfo`** — CPU name (`HKLM\HARDWARE\DESCRIPTION\System\
  CentralProcessor\0\ProcessorNameString`) and logical core count, used to auto-select
  the chip profile for labelling.
- **Power** — not wired; confirmed unavailable from userspace (§3).
- Emit a single `sensor-update` Tauri event each tick with the merged snapshot.
- Strict real-only contract: any field with no real source is `None` → UI shows "—".

---

## 7. IMPORTANT: COM/IWbemServices fails under Tauri — use PowerShell

The Rust `wmi` crate path (`COMLibrary` → `WMIConnection` → `raw_query` via
`IWbemServices::ExecQuery`) **fails with `WBEM_E_NOT_FOUND` (0x80041002)** when
called from inside the Tauri process, even though the identical query succeeds:
- from PowerShell (`Get-CimInstance`), and
- from a standalone Rust binary (both console *and* `windows_subsystem = "windows"`),
- on both the main thread *and* a `thread::spawn` worker.

The failure occurs at the `ExecQuery` step (COM init and namespace connection both
succeed). It is a Tauri-environment COM/WMI interaction we could not resolve. The
**working solution** is the PowerShell-backed provider (`sensors/powershell.rs`):
it shells out to `powershell.exe -Command Get-CimInstance … | ConvertTo-Json`
once per poll tick and parses the JSON. This is the *exact* mechanism Phase 0 used
and produces complete, real data (CPU identity, all 36 thermal zones, per-core load).

Trade-off: one `powershell.exe` child process per ~2s tick (~50–150ms). Acceptable
for a monitor; the COM path is retained in `sensors/provider.rs` (excluded from the
build) for a future dedicated-subprocess or kernel-driver approach.

