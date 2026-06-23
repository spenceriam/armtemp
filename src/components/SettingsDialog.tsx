import { useState } from "react";
import {
  AppSettings,
  UiStyle,
  TrayMode,
  TrayStyle,
  OverheatAction,
  TempUnit,
  ThemeChoice,
  TaskbarMode,
} from "../app/types";
import { ChipsList } from "./ChipsList";

type Tab = "general" | "display" | "notif" | "taskbar" | "over" | "about";

interface Props {
  settings: AppSettings;
  update: (patch: Partial<AppSettings>) => void;
  tjmax: number;
  onClose: () => void;
}

// The 6-tab settings dialog, matching the CoreTemp layout. The Temperature unit
// lives in the General tab (NOT in the main-window toolbar).
export function SettingsDialog({ settings, update, tjmax, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const tabs: [Tab, string][] = [
    ["general", "General"],
    ["display", "Display"],
    ["notif", "Notification area"],
    ["taskbar", "Windows Taskbar"],
    ["over", "Overheat protection"],
    ["about", "About"],
  ];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">⚙ ARMTEMP — Settings</span>
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
            {tab === "general" && <GeneralTab settings={settings} update={update} />}
            {tab === "display" && <DisplayTab settings={settings} update={update} />}
            {tab === "notif" && <NotifTab settings={settings} update={update} />}
            {tab === "taskbar" && <TaskbarTab settings={settings} update={update} />}
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

function SectionHead({ children }: { children: React.ReactNode }) {
  return <div className="subhead">{children}</div>;
}

function Seg<T extends string | number>({
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
          key={String(v)}
          className={`seg-btn ${value === v ? "active" : ""}`}
          onClick={() => onChange(v)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function RadioGroup<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: [T, string, string?][];
  onChange: (v: T) => void;
}) {
  return (
    <div className="radio-group">
      {options.map(([v, label, sub]) => (
        <button
          key={v}
          className={`radio ${value === v ? "active" : ""}`}
          onClick={() => onChange(v)}
        >
          <div className={`radio-dot ${value === v ? "active" : ""}`} />
          <div>
            <div className="radio-label">{label}</div>
            {sub && <div className="radio-sub">{sub}</div>}
          </div>
        </button>
      ))}
    </div>
  );
}

// --- tabs ---

function GeneralTab({
  settings,
  update,
}: {
  settings: AppSettings;
  update: (p: Partial<AppSettings>) => void;
}) {
  return (
    <>
      <h3>General</h3>
      <SectionHead>Startup</SectionHead>
      <Row title="Start ARMTEMP when Windows starts" sub="Launch automatically at sign-in">
        <Toggle on={settings.startWithWindows} onClick={() => update({ startWithWindows: !settings.startWithWindows })} />
      </Row>
      <Row title="Start minimized to the notification area" sub="Go straight to the tray">
        <Toggle on={settings.startMinimized} onClick={() => update({ startMinimized: !settings.startMinimized })} />
      </Row>

      <SectionHead>Window</SectionHead>
      <Row title="Close to the notification area" sub="Closing the window keeps it running">
        <Toggle on={settings.closeToTray} onClick={() => update({ closeToTray: !settings.closeToTray })} />
      </Row>
      <Row title="Always on top" sub="Keep the window above others">
        <Toggle on={settings.alwaysOnTop} onClick={() => update({ alwaysOnTop: !settings.alwaysOnTop })} />
      </Row>
      <Row title="Hide when minimized" sub="Minimizing sends it to the tray">
        <Toggle on={settings.hideWhenMinimized} onClick={() => update({ hideWhenMinimized: !settings.hideWhenMinimized })} />
      </Row>

      {/* Temperature unit — its primary home. Not in the main toolbar. */}
      <SectionHead>Temperature unit</SectionHead>
      <Row title="Display readings in" sub="Applies to the table, status bar, and tray">
        <Seg<TempUnit>
          value={settings.tempUnit}
          options={[
            ["C", "°C"],
            ["F", "°F"],
          ]}
          onChange={(v) => update({ tempUnit: v })}
        />
      </Row>

      <SectionHead>Polling interval</SectionHead>
      <div className="slider-block">
        <div className="slider-head">
          <span className="row-title">Refresh rate</span>
          <span className="slider-val">{settings.pollingIntervalMs} ms</span>
        </div>
        <input
          type="range"
          min={500}
          max={5000}
          step={250}
          value={settings.pollingIntervalMs}
          onChange={(e) => update({ pollingIntervalMs: Number(e.target.value) })}
          className="slider"
        />
        <div className="slider-hint">500 ms — 5000 ms (lower = more responsive, higher = less CPU)</div>
      </div>
    </>
  );
}

function DisplayTab({
  settings,
  update,
}: {
  settings: AppSettings;
  update: (p: Partial<AppSettings>) => void;
}) {
  return (
    <>
      <h3>Display</h3>
      <SectionHead>Main window layout</SectionHead>
      <RadioGroup<UiStyle>
        value={settings.uiStyle}
        onChange={(v) => update({ uiStyle: v })}
        options={[
          ["classic", "Classic", "Table with Core / Temp / Low / High / Load"],
          ["cards", "Cards", "Grid of per-core tiles"],
          ["dashboard", "Dashboard", "Package graph + cards"],
        ]}
      />

      <SectionHead>Window theme</SectionHead>
      <Row title="Color scheme" sub="System follows the Windows theme">
        <Seg<ThemeChoice>
          value={settings.theme}
          options={[
            ["system", "System"],
            ["dark", "Dark"],
            ["light", "Light"],
          ]}
          onChange={(v) => update({ theme: v })}
        />
      </Row>

      <SectionHead>Window zoom</SectionHead>
      <Row title="Scale the window" sub="For HiDPI or readability">
        <Seg
          value={settings.zoom}
          options={[
            [75, "75%"],
            [100, "100%"],
            [125, "125%"],
          ]}
          onChange={(v) => update({ zoom: v as 75 | 100 | 125 })}
        />
      </Row>

      <SectionHead>Status bar & colors</SectionHead>
      <Row title="Show status bar" sub="CPU Temp / Avg / Low / High in the footer">
        <Toggle on={settings.statusBarOn} onClick={() => update({ statusBarOn: !settings.statusBarOn })} />
      </Row>
      <Row title="Color-code temperatures" sub="Green → yellow → orange → red by Tj. Max proximity">
        <Toggle on={settings.colorCodeTemps} onClick={() => update({ colorCodeTemps: !settings.colorCodeTemps })} />
      </Row>
    </>
  );
}

function NotifTab({
  settings,
  update,
}: {
  settings: AppSettings;
  update: (p: Partial<AppSettings>) => void;
}) {
  return (
    <>
      <h3>Notification area</h3>
      <Row title="Show ARMTEMP in the notification area" sub="Live tray icon">
        <Toggle on={settings.trayOn} onClick={() => update({ trayOn: !settings.trayOn })} />
      </Row>

      <SectionHead>Tray icon displays</SectionHead>
      <RadioGroup<TrayMode>
        value={settings.trayMode}
        onChange={(v) => update({ trayMode: v })}
        options={[
          ["highest", "Highest core", "Hottest single reading"],
          ["average", "Average", "Mean across all cores"],
          ["all", "All cores", "One icon per core"],
          ["package", "Package", "Overall CPU package"],
        ]}
      />

      <SectionHead>Icon style</SectionHead>
      <Row title="Tray icon shape" sub="Visual treatment of the temperature badge">
        <Seg<TrayStyle>
          value={settings.trayStyle}
          options={[
            ["rounded", "Rounded"],
            ["badge", "Badge"],
            ["plain", "Plain"],
          ]}
          onChange={(v) => update({ trayStyle: v })}
        />
      </Row>

      <SectionHead>Tray tooltip</SectionHead>
      <Row title="Show all core temps on hover" sub="Avg · High · Low in the tooltip">
        <Toggle on={settings.trayTooltipAllCores} onClick={() => update({ trayTooltipAllCores: !settings.trayTooltipAllCores })} />
      </Row>
    </>
  );
}

function TaskbarTab({
  settings,
  update,
}: {
  settings: AppSettings;
  update: (p: Partial<AppSettings>) => void;
}) {
  return (
    <>
      <h3>Windows Taskbar</h3>
      <Row title="Show temperature on the taskbar button" sub="Live reading on the ARMTEMP taskbar icon">
        <Toggle on={settings.taskbarOn} onClick={() => update({ taskbarOn: !settings.taskbarOn })} />
      </Row>

      <SectionHead>Taskbar shows</SectionHead>
      <Row title="Reading mode" sub="Per-core or a combined average">
        <Seg<TaskbarMode>
          value={settings.taskbarMode}
          options={[
            ["per-core", "Per-core"],
            ["average", "Average"],
          ]}
          onChange={(v) => update({ taskbarMode: v })}
        />
      </Row>

      <SectionHead>Button appearance</SectionHead>
      <Row title="Use accent color background" sub="Blue tile behind the temperature">
        <Toggle on={settings.taskbarAccent} onClick={() => update({ taskbarAccent: !settings.taskbarAccent })} />
      </Row>
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
        <div className="slider-hint">70°C ────────────────── {Math.min(105, Math.round(tjmax))}°C (Tj. Max)</div>
        <div className="slider-hint">Recommended: ~5°C below Tj. Max ({tjmax}°C)</div>
      </div>

      <SectionHead>When threshold exceeded</SectionHead>
      <RadioGroup<OverheatAction>
        value={settings.overheatAction}
        onChange={(v) => update({ overheatAction: v })}
        options={[
          ["notify", "Show a notification"],
          ["sleep", "Put the PC to sleep"],
          ["shutdown", "Shut down the PC"],
        ]}
      />
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
          <div className="about-version">Version 0.1.0 · ARM64 build · Tauri</div>
        </div>
      </div>
      <SectionHead>Detected processor</SectionHead>
      <ChipsList />
      <p className="about-note">
        Temperatures read from on-die thermal sensors via ACPI thermal zones. Per-core
        temps are real zone readings mapped to cores (not true per-core sensors on this
        firmware). ARMTEMP is an independent monitoring utility and is not affiliated
        with any silicon vendor.
      </p>
    </>
  );
}
