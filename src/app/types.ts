// Shared types mirroring the Rust SensorSnapshot (see src-tauri/src/sensors/types.rs).

export type CoreKind = "performance" | "efficiency";

export interface CoreReading {
  index: number;
  kind: CoreKind;
  load: number | null;
  temp_c: number | null;
  min_c: number | null;
  max_c: number | null;
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
  tjmax_c: number;
  package_c: number | null;
  average_c: number | null;
  zones: ZoneReading[];
  cores: CoreReading[];
  clock_mhz: number | null;
  max_clock_mhz: number | null;
  power_w: number | null;
  tick: number;
}

export type TempUnit = "C" | "F";
export type UiStyle = "classic" | "cards" | "dashboard";
export type TrayMode = "all" | "highest" | "average" | "package";
export type TrayStyle = "rounded" | "badge" | "plain";
export type OverheatAction = "notify" | "sleep" | "shutdown";

export interface AppSettings {
  startWithWindows: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  followTheme: boolean;
  tempUnit: TempUnit;
  uiStyle: UiStyle;
  toolbarMode: "per-core" | "average";
  zoom: 75 | 100 | 125;
  trayOn: boolean;
  trayMode: TrayMode;
  trayStyle: TrayStyle;
  overheatOn: boolean;
  overheatThreshold: number;
  overheatAction: OverheatAction;
  theme: "dark" | "light";
}

export const DEFAULT_SETTINGS: AppSettings = {
  startWithWindows: false,
  startMinimized: false,
  closeToTray: true,
  followTheme: true,
  tempUnit: "C",
  uiStyle: "classic",
  toolbarMode: "per-core",
  zoom: 100,
  trayOn: true,
  trayMode: "highest",
  trayStyle: "rounded",
  overheatOn: false,
  overheatThreshold: 95,
  overheatAction: "notify",
  theme: "dark",
};
