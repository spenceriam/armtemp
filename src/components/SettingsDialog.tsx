import { useState } from "react";
import { AppSettings, UiStyle, TrayMode, TrayStyle, OverheatAction, TempUnit } from "../app/types";
import { ChipsList } from "./ChipsList";

type Tab = "general" | "display" | "notif" | "over" | "about";

interface Props {
  settings: AppSettings;
  update: (patch: Partial<AppSettings>) => void;
  tjmax: number;
  onClose: () => void;
}

// The 5-tab settings dialog, ported screen-for-screen from the Clod design.
export function SettingsDialog({ settings, update, tjmax, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const tabs: [Tab, string][] = [
    ["general", "General"],
    ["display", "Display"],
    ["notif", "Notification area"],
    ["over", "Overheat protection"],
    ["about", "About"],
  ];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">ARMTEMP — Settings</span>
          <div className="modal-spacer" />
          <button className="win-btn close-btn" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">
          <div className="settings-nav">
            {tabs.map(([id, label]) => (
              <button
                key={id}
                className={`nav-item ${tab === id ? "active" : ""}`}
                onClick={() => setTab(id)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="settings-content">
            {tab === "general" && (
              <GeneralTab settings={settings} update={update} />
            )}
            {tab === "display" && <DisplayTab settings={settings} update={update} />}
            {tab === "notif" && <NotifTab settings={settings} update={update} />}
            {tab === "over" && <OverheatTab settings={settings} update={update} tjmax={tjmax} />}
            {tab === "about" && <AboutTab />}
          </div>
        </div>
      </div>
    </div>
  );
}

// --- reusable controls ---

function Toggle({ on, onClick }: { on: boolean; onClick: () => void }) {
  return (
    <div className={`switch ${on ? "on" : ""}`} onClick={onClick}>
      <div className="switch-knob" />
    </div>
  );
}

function Row({ title, sub, children }: { title: string; sub: string; children: React.ReactNode }) {
  return (
    <div className="settings-row">
      <div>
        <div className="row-title">{title}</div>
        <div className="row-sub">{sub}</div>
      </div>
      {children}
    </div>
  );
}

function Seg<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
}) {
  return (
    <div className="seg">
      {options.map(([v, label]) => (
        <button
          key={v}
          className={`seg-btn ${value === v ? "active" : ""}`}
          onClick={() => onChange(v)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

// --- tabs ---

function GeneralTab({ settings, update }: { settings: AppSettings; update: (p: Partial<AppSettings>) => void }) {
  return (
    <>
      <h3>General</h3>
      <Row title="Start ARMTEMP with Windows" sub="Launch automatically at sign-in">
        <Toggle on={settings.startWithWindows} onClick={() => update({ startWithWindows: !settings.startWithWindows })} />
      </Row>
      <Row title="Start minimized to tray" sub="Go straight to the notification area">
        <Toggle on={settings.startMinimized} onClick={() => update({ startMinimized: !settings.startMinimized })} />
      </Row>
      <Row title="Close to notification area" sub="Closing the window keeps it running in the tray">
        <Toggle on={settings.closeToTray} onClick={() => update({ closeToTray: !settings.closeToTray })} />
      </Row>
      <Row title="Follow Windows theme" sub="Mirror the system light / dark mode">
        <Toggle on={settings.followTheme} onClick={() => update({ followTheme: !settings.followTheme })} />
      </Row>
      <Row title="Temperature unit" sub="Display readings in Celsius or Fahrenheit">
        <Seg<TempUnit>
          value={settings.tempUnit}
          options={[
            ["C", "°C"],
            ["F", "°F"],
          ]}
          onChange={(v) => update({ tempUnit: v })}
        />
      </Row>
    </>
  );
}

function DisplayTab({ settings, update }: { settings: AppSettings; update: (p: Partial<AppSettings>) => void }) {
  return (
    <>
      <h3>Display</h3>
      <Row title="Main window style" sub="Layout used for the core readout">
        <Seg<UiStyle>
          value={settings.uiStyle}
          options={[
            ["classic", "Classic"],
            ["cards", "Cards"],
            ["dashboard", "Dashboard"],
          ]}
          onChange={(v) => update({ uiStyle: v })}
        />
      </Row>
      <Row title="Show temperature in toolbar" sub="Per-core readings or a combined average">
        <Seg
          value={settings.toolbarMode}
          options={[
            ["per-core", "Per-core"],
            ["average", "Average"],
          ]}
          onChange={(v) => update({ toolbarMode: v as "per-core" | "average" })}
        />
      </Row>
      <Row title="Window zoom" sub="Scale the ARMTEMP window">
        <Seg
          value={String(settings.zoom) as "75" | "100" | "125"}
          options={[
            ["75", "75%"],
            ["100", "100%"],
            ["125", "125%"],
          ]}
          onChange={(v) => update({ zoom: Number(v) as 75 | 100 | 125 })}
        />
      </Row>
    </>
  );
}

function NotifTab({ settings, update }: { settings: AppSettings; update: (p: Partial<AppSettings>) => void }) {
  const modes: [TrayMode, string, string][] = [
    ["all", "All cores", "One icon per core"],
    ["highest", "Highest core", "Hottest core only — less clutter"],
    ["average", "Average", "Mean across all cores"],
    ["package", "Package", "Overall CPU package temperature"],
  ];
  return (
    <>
      <h3>Notification area</h3>
      <Row title="Show temperature icons in tray" sub="Live readouts in the taskbar corner">
        <Toggle on={settings.trayOn} onClick={() => update({ trayOn: !settings.trayOn })} />
      </Row>
      <div className="subhead">Tray icon mode</div>
      {modes.map(([id, label, sub]) => (
        <button
          key={id}
          className={`radio ${settings.trayMode === id ? "active" : ""}`}
          onClick={() => update({ trayMode: id })}
        >
          <div className={`radio-dot ${settings.trayMode === id ? "active" : ""}`} />
          <div>
            <div className="radio-label">{label}</div>
            <div className="radio-sub">{sub}</div>
          </div>
        </button>
      ))}
      <div className="subhead">Icon style</div>
      <Seg<TrayStyle>
        value={settings.trayStyle}
        options={[
          ["rounded", "Rounded"],
          ["badge", "Badge"],
          ["plain", "Plain"],
        ]}
        onChange={(v) => update({ trayStyle: v })}
      />
    </>
  );
}

function OverheatTab({
  settings,
  update,
  tjmax,
}: {
  settings: AppSettings;
  update: (p: Partial<AppSettings>) => void;
  tjmax: number;
}) {
  const actions: [OverheatAction, string][] = [
    ["notify", "Show a notification"],
    ["sleep", "Put the PC to sleep"],
    ["shutdown", "Shut down the PC"],
  ];
  return (
    <>
      <h3>Overheat protection</h3>
      <Row title="Enable overheat protection" sub="Take action when a core gets too hot">
        <Toggle on={settings.overheatOn} onClick={() => update({ overheatOn: !settings.overheatOn })} />
      </Row>
      <div className="slider-block">
        <div className="slider-head">
          <span className="row-title">Warning threshold</span>
          <span className="slider-val">{settings.overheatThreshold}°C</span>
        </div>
        <input
          type="range"
          min={70}
          max={Math.min(105, Math.round(tjmax))}
          step={1}
          value={settings.overheatThreshold}
          onChange={(e) => update({ overheatThreshold: Number(e.target.value) })}
          className="slider"
        />
        <div className="slider-hint">Recommended: ~5° below Tj. Max ({tjmax}°C)</div>
      </div>
      <div className="subhead">Action when exceeded</div>
      {actions.map(([id, label]) => (
        <button
          key={id}
          className={`radio ${settings.overheatAction === id ? "active" : ""}`}
          onClick={() => update({ overheatAction: id })}
        >
          <div className={`radio-dot ${settings.overheatAction === id ? "active" : ""}`} />
          <span className="radio-label">{label}</span>
        </button>
      ))}
    </>
  );
}

function AboutTab() {
  return (
    <>
      <h3>About</h3>
      <div className="about-head">
        <div className="about-logo">°</div>
        <div>
          <div className="about-name">ARMTEMP</div>
          <div className="about-version">Version 2.0.4 · ARM64 build · Tauri</div>
        </div>
      </div>
      <div className="subhead">Detected processor</div>
      <ChipsList />
      <p className="about-note">
        Per-core temperatures read from on-die thermal sensors via ACPI thermal
        zones. ARMTEMP is an independent monitoring utility and is not affiliated
        with any silicon vendor.
      </p>
    </>
  );
}
