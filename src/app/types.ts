// Shared types mirroring the Rust SensorSnapshot (see src-tauri/src/sensors/types.ts).

export type CoreKind = "performance" | "efficiency";

export interface CoreReading {
  index: number;
  kind: CoreKind;
  load: number | null;
  load_min: number | null;
  load_max: number | null;
  load_avg: number | null;
}

export interface ZoneReading {
  name: string;
  temp_c: number;
  throttled: boolean;
}

export interface SensorSnapshot {
  chip_name: string;
  chip_model: string;
  core_thread: string;
  platform: string;
  lithography: string;
  tdp_w: number | null;
  tjmax_c: number;
  package_c: number | null;
  package_min_c: number | null;
  package_max_c: number | null;
  package_avg_c: number | null;
  zones: ZoneReading[];
  cores: CoreReading[];
  clock_mhz: number | null;
  base_clock_mhz: number | null;
  max_clock_mhz: number | null;
  bus_speed_mhz: number | null;
  power_w: number | null;
  cpu_identifier: string | null;
  detection_basis: string;
  tick: number;
}

export type TempUnit = "C" | "F";
export type UiStyle = "classic" | "cards" | "dashboard";
export type TrayStyle = "rounded" | "badge" | "plain";
export type OverheatAction = "notify" | "sleep" | "shutdown";
export type ThemeChoice = "system" | "dark" | "light";

export interface AppSettings {
  // General
  tempUnit: TempUnit;
  startWithWindows: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  alwaysOnTop: boolean;
  hideWhenMinimized: boolean;
  pollingIntervalMs: number;
  // Display
  uiStyle: UiStyle;
  theme: ThemeChoice;
  statusBarOn: boolean;
  colorCodeTemps: boolean;
  // Notification Area
  trayOn: boolean;
  trayStyle: TrayStyle;
  trayTooltipAllCores: boolean;
  trayBoldFont: boolean;
  trayDegreeSymbol: boolean;
  // Windows Taskbar
  taskbarOn: boolean;
  taskbarAccent: boolean;
  // Overheat
  overheatOn: boolean;
  overheatThreshold: number;
  overheatAction: OverheatAction;
}

export const DEFAULT_SETTINGS: AppSettings = {
  // General — unit lives here (NOT in the main-window toolbar)
  tempUnit: "C",
  startWithWindows: false,
  startMinimized: false,
  closeToTray: true,
  alwaysOnTop: false,
  hideWhenMinimized: false,
  pollingIntervalMs: 1500,
  // Display
  uiStyle: "classic",
  theme: "dark",
  statusBarOn: true,
  colorCodeTemps: true,
  // Notification Area
  trayOn: true,
  trayStyle: "plain",
  trayTooltipAllCores: true,
  trayBoldFont: true,
  trayDegreeSymbol: false,
  // Windows Taskbar
  taskbarOn: true,
  taskbarAccent: true,
  // Overheat
  overheatOn: false,
  overheatThreshold: 95,
  overheatAction: "notify",
};
