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
  `group,core`, e.g. `0,3`), `% Processor Performance` per core, and `Processor
  Frequency` on `_Total`. **Speed** is computed per-core (`% Processor Performance ×
  that core's registry `~MHz`) and the fastest core wins — the `_Total` counter alone
  blends both P/E clusters into a meaningless average on a heterogeneous chip (see
  issue #2: an idle Efficiency cluster dragged a busy X2's reported Speed well below
  its real Prime-cluster clock); `_Total` is kept only as a fallback if the per-core
  counters aren't available.
- **Registry + `GetSystemInfo`** (`sensors/identity.rs`) — `ProcessorNameString`,
  `Identifier` (MIDR-derived: "Model 1" = 1st-gen Oryon/X1, "Model 2" = Oryon V3/X2),
  `VendorIdentifier`, per-core `~MHz` (each core's cluster rated/boost clock — the same
  value CPU-Z's "Original Processor Frequency" and HWiNFO read), and logical core count.
- **Chip detection** (`sensors/chips.rs::match_profile`) is layered, since issue #2
  showed some OEM firmware (Surface, Snapdragon X2 Elite) reports a bare marketing name
  with no SKU token in it at all — token matching alone silently produced "Unknown X2
  Elite SKU" for it every time:
  1. Exact SKU token in the name (e.g. `X2E78100`) — used when present.
  2. Family/subfamily from the name text ("X2 Elite", "X2 Elite Extreme", "X2 Plus", …),
     or — if the name has no usable text — the registry `Identifier`'s Oryon generation
     + a Qualcomm vendor check.
  3. Within that family, the exact SKU inferred from real core count + real rated clock
     (`~MHz`) — nearest-clock match within a tolerance, since every family+core-count
     group's neighboring SKUs are ≥300 MHz apart. X2E-80-100 and X2E-84-100 publish
     identical CPU-visible specs (same 4.7 GHz boost, cache, core split) and are
     reported as one honest combined "X2E-80/84-100" label rather than a guess.
  4. Family recognized but no SKU candidate fits (unreleased part): label the family
     honestly with `boost_ghz` from the real measured clock — never "Generic".
  5. A known non-Snapdragon-X vendor/family (legacy Qualcomm Kryo chips — Snapdragon
     835/850/7c family/8c family/8cx family, Microsoft SQ1-3; Broadcom; MediaTek;
     NVIDIA) but no specific SKU token matched: honest vendor/family label, never an
     invented spec.
  6. Nothing recognized at all: the real `VendorIdentifier` (e.g. "Broadcom CPU") or
     bare "ARM64 CPU" — never "Snapdragon Generic" for a chip that isn't one.
  Real P/E core counts from the OS topology query (below) override any static table
  cluster split whenever they're available and consistent with the detected core count.
- **Core-tier vocabulary** (`ChipProfile::tier_badge`, used for every per-core "P"/"E"
  badge in the UI): the label depends on the real silicon, not a one-size-fits-all
  Performance/Efficiency split.
  - **Snapdragon X1 family**: every core badges **"P"** ("Performance core") — X1 has no
    efficiency tier at all. All cores are identical Oryon cores; the two clusters differ
    only in clock cap (confirmed via `GetLogicalProcessorInformationEx`'s `EfficiencyClass`
    on this dev machine, X1P-64-100: 4 cores capped at 2976 MHz report `EfficiencyClass=0`,
    6 cores at 3418 MHz report `EfficiencyClass=1` — real data, but "Efficiency" is the
    wrong noun for a chip with no efficiency cores).
  - **Snapdragon X2 family**: badges **"P"**/**"P2"**, tooltips "Prime core"/"Performance
    core" — Qualcomm's own X2 vocabulary; still no "Efficiency" cluster name.
  - **Legacy Kryo-based Qualcomm chips** (835/850/7c/8c/8cx families, Microsoft SQ1-3),
    and any other genuinely hybrid/unrecognized ARM64 chip: badges **"P"**/**"E"** — this
    *is* accurate vocabulary for those, since they're real big.LITTLE designs (e.g.
    Kryo "Gold"/"Silver", or Cortex-X1C/A78C on 8cx Gen 3 / SQ3).
- **Power** — not wired; confirmed unavailable from userspace (§3).
- Emit a single `sensor-update` Tauri event each tick with the merged snapshot.
- Strict real-only contract: any field with no real source is `None` → UI shows "—". This
  matters more now that detection covers non-Qualcomm boards (Raspberry Pi under
  Windows-on-ARM, etc.): the `Thermal Zone Information` PDH object this app relies on for
  temperature is Qualcomm/Surface-firmware-specific (§1) and may simply not exist on other
  vendors' boards — package temperature (and TDP/lithography where unpublished) degrade to
  an honest "—" there rather than fabricating a reading.

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

