# ARMTEMP

A native **temperature monitor for Snapdragon X / X2** processors (X1/X1P/X1E, X2/X2P/X2E — Qualcomm Oryon) on **Windows on ARM (ARM64)**.

ARMTEMP reads real on-die thermal sensors and displays package + multi-zone temperatures, per-core utilization, and power — recreating the [Core Temp](https://www.alcpu.com/CoreTemp/) experience for the Snapdragon X family. Built as a small native ARM64 binary (Tauri 2 + Rust + React), with a live system-tray icon, mini-mode, and overheat protection.

> This is a from-scratch native app ported from a Clod/Design-Component mockup (`claude-design-output/`), which was a *simulated* Windows-desktop preview. All live data here is **real**; no readings are fabricated. See [`SENSORS.md`](./SENSORS.md) for the full sensor discovery report.

---

## Status

**Working:** native ARM64 build, real telemetry (package temp via ACPI thermal zones, per-core load, CPU identity), 3 UI layouts (Classic / Cards / Dashboard), mini-mode, 5-tab settings dialog, live tray icon with temperature, context menu, close-to-tray, overheat-protection notifications. MSI + NSIS installers build.

**Honest limitations (by firmware, not by choice):**
- **Per-core temperatures** are not exposed by any userspace surface on Snapdragon X. The firmware exposes ~17 valid *zone* temperatures (which ARMTEMP shows) plus real per-core *load*. True per-core temps would require a signed kernel driver or private Surface/Qualcomm SMF IOCTLs — a future workstream.
- **Voltage** is not exposed; the field shows `—`.

---

## How it reads sensors (the important part)

Reading temperatures on Snapdragon X under Windows is genuinely hard — the standard WMI classes (`MSAcpi_ThermalZoneTemperature`, `Win32_TemperatureProbe`) return **nothing** on this firmware, and LibreHardwareMonitor has no native Oryon support.

ARMTEMP's working data source is the **`Thermal Zone Information`** performance-counter object, which exposes ACPI thermal zones (`\_SB.TZxx`). Sentinels/inactive zones (≤ 0 °C) are filtered out; the rest are converted from Kelvin to °C. The hottest valid zone is reported as the package temperature.

**Implementation note:** ARMTEMP reads these counters natively via the Windows **PDH** (Performance Data Helper) API (`pdh.dll`) — no subprocess, no COM/WMI. This also sidesteps the failure the Rust `wmi` crate's COM/`IWbemServices` path hit when called from inside a Tauri process (`WBEM_E_NOT_FOUND`; see [`SENSORS.md`](./SENSORS.md) §7 for that history). A dedicated worker thread owns the PDH query handles and collects once per poll tick (~1.5–2s); the same counters also yield a genuinely *live* CPU frequency, unlike `Win32_Processor.CurrentClockSpeed`, which mirrors the static max on this firmware.

---

## Build & run

**Prerequisites:** Rust (`aarch64-pc-windows-msvc` target), Node.js 20+, on a Windows on ARM machine (or cross-compile target).

```bash
# Install frontend deps
npm install

# Dev mode (hot-reload frontend + native backend)
npm run tauri dev

# Production build -> native ARM64 .exe + MSI/NSIS installers
npm run tauri build
# Output: src-tauri/target/release/armtemp.exe
#         src-tauri/target/release/bundle/{msi,nsis}/
```

Output is a **native ARM64** (`AA64`) executable (~3.8 MB), no emulation.

---

## Project layout

```
armtemp/
├── claude-design-output/      # The original Clod mockup (simulated, HTML/JS) — reference only
├── src/                       # React + TypeScript frontend
│   ├── app/                   # types, theme tokens, hooks (settings, sensors)
│   ├── components/            # TitleBar, ProcessorInfo, layouts/, SettingsDialog, MiniMode…
│   └── styles.css             # Design tokens ported from the mockup
├── src-tauri/                 # Rust + Tauri backend
│   └── src/
│       ├── lib.rs             # App wiring, poll loop, tray, IPC commands
│       └── sensors/           # Real telemetry: powershell.rs (primary), chips.rs, tray.rs, types.rs
├── tools/                     # Sensor probe scripts (Phase 0 artifacts) + icon generator
└── SENSORS.md                 # Full sensor discovery report (what works, what doesn't, why)
```

## Design parity

Recreates Core Temp's actual desktop layout — native title bar and window chrome (no custom-drawn frame, no transparency/blur), a real File/Options/Tools/Help menu bar, a Select CPU combo, Win32-style etched group boxes with sunken read-only value fields for Processor Information, and a Temperature Readings table with colored temperature text (no dots/bars) — while keeping our own dark/light theming (real Core Temp has none), the accent + shade color helpers, the green→yellow→orange→red temperature color scale, the Cards grid, the Dashboard sparkline, a chrome-less mini-mode, a classic tabbed Settings dialog (General / Display / Notification Area / Windows Taskbar — native checkboxes, OK/Cancel/Apply) with separate Overheat protection (Options menu) and About (Help menu) dialogs, and the tray icon with its context menu. Dropped (provided by Windows itself now): the simulated desktop/taskbar/Start-menu chrome and the random data generator.

---

*ARMTEMP is an independent monitoring utility and is not affiliated with any silicon vendor.*
