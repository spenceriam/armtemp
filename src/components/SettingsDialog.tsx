import { useRef, useState } from "react";
import { AppSettings, UiStyle, TrayStyle, ThemeChoice } from "../app/types";

type Tab = "general" | "display" | "notif" | "taskbar";

interface Props {
  settings: AppSettings;
  update: (patch: Partial<AppSettings>) => void;
  onClose: () => void;
}

// Classic Win32-style tabbed Settings dialog, modeled on real Core Temp:
// top tab strip, native checkboxes/radios/selects inside etched group boxes,
// and OK / Cancel / Apply buttons. Changes apply live (that's how the app
// works everywhere); Cancel reverts to the state captured when the dialog
// opened (or when Apply was last pressed) — genuine dialog semantics.
export function SettingsDialog({ settings, update, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const baseline = useRef<AppSettings>(settings);

  const tabs: [Tab, string][] = [
    ["general", "General"],
    ["display", "Display"],
    ["notif", "Notification Area"],
    ["taskbar", "Windows Taskbar"],
  ];

  const cancel = () => {
    update(baseline.current);
    onClose();
  };
  const apply = () => {
    baseline.current = settings;
  };

  return (
    <div className="modal-overlay" onClick={cancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">Settings</span>
          <div className="modal-spacer" />
          <button className="win-btn close-btn" onClick={cancel}>
            ✕
          </button>
        </div>

        <div className="tabstrip">
          {tabs.map(([id, label]) => (
            <button
              key={id}
              className={`tab ${tab === id ? "active" : ""}`}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          ))}
        </div>

        <div className="tab-panel">
          {tab === "general" && <GeneralTab settings={settings} update={update} />}
          {tab === "display" && <DisplayTab settings={settings} update={update} />}
          {tab === "notif" && <NotifTab settings={settings} update={update} />}
          {tab === "taskbar" && <TaskbarTab settings={settings} update={update} />}
        </div>

        <div className="dlg-buttons">
          <button className="btn" onClick={onClose}>
            OK
          </button>
          <button className="btn" onClick={cancel}>
            Cancel
          </button>
          <button className="btn" onClick={apply}>
            Apply
          </button>
        </div>
      </div>
    </div>
  );
}

// --- native-style controls ---

function Check({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="chk-row">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  );
}

function SelectRow<T extends string | number>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
}) {
  return (
    <label className="select-row">
      <span>{label}</span>
      <select
        value={String(value)}
        onChange={(e) => {
          const raw = e.target.value;
          const match = options.find(([v]) => String(v) === raw);
          if (match) onChange(match[0]);
        }}
      >
        {options.map(([v, text]) => (
          <option key={String(v)} value={String(v)}>
            {text}
          </option>
        ))}
      </select>
    </label>
  );
}

function Group({ legend, children }: { legend: string; children: React.ReactNode }) {
  return (
    <div className="groupbox dlg-group">
      <span className="groupbox-legend">{legend}</span>
      {children}
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
      <Group legend="Startup">
        <Check
          label="Start ARMtemp with Windows"
          checked={settings.startWithWindows}
          onChange={(v) => update({ startWithWindows: v })}
        />
        <Check
          label="Start ARMtemp minimized"
          checked={settings.startMinimized}
          onChange={(v) => update({ startMinimized: v })}
        />
      </Group>
      <Group legend="Polling">
        <label className="select-row">
          <span>Temperature polling interval</span>
          <span className="num-wrap">
            <input
              type="number"
              className="num-input"
              min={500}
              max={5000}
              step={250}
              value={settings.pollingIntervalMs}
              onChange={(e) => {
                const n = Number(e.target.value);
                if (Number.isFinite(n)) update({ pollingIntervalMs: n });
              }}
            />
            <span className="num-unit">ms</span>
          </span>
        </label>
      </Group>
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
      <Group legend="Settings">
        <Check
          label="Display temperatures in Fahrenheit"
          checked={settings.tempUnit === "F"}
          onChange={(v) => update({ tempUnit: v ? "F" : "C" })}
        />
        <Check
          label="Close ARMtemp to the notification area"
          checked={settings.closeToTray}
          onChange={(v) => update({ closeToTray: v })}
        />
        <Check
          label="Hide when minimized"
          checked={settings.hideWhenMinimized}
          onChange={(v) => update({ hideWhenMinimized: v })}
        />
        <Check
          label="Always on top"
          checked={settings.alwaysOnTop}
          onChange={(v) => update({ alwaysOnTop: v })}
        />
        <Check
          label="Show status bar"
          checked={settings.statusBarOn}
          onChange={(v) => update({ statusBarOn: v })}
        />
        <Check
          label="Color-code temperatures"
          checked={settings.colorCodeTemps}
          onChange={(v) => update({ colorCodeTemps: v })}
        />
      </Group>
      <Group legend="Appearance">
        <SelectRow<ThemeChoice>
          label="Color scheme"
          value={settings.theme}
          options={[
            ["system", "System"],
            ["dark", "Dark"],
            ["light", "Light"],
          ]}
          onChange={(v) => update({ theme: v })}
        />
        <SelectRow<UiStyle>
          label="View"
          value={settings.uiStyle}
          options={[
            ["classic", "Classic"],
            ["cards", "Cards"],
            ["dashboard", "Dashboard"],
          ]}
          onChange={(v) => update({ uiStyle: v })}
        />
      </Group>
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
      <Group legend="Notification area icon">
        <Check
          label="Show ARMtemp in the notification area"
          checked={settings.trayOn}
          onChange={(v) => update({ trayOn: v })}
        />
        <Check
          label="Show average load in the tooltip"
          checked={settings.trayTooltipAllCores}
          onChange={(v) => update({ trayTooltipAllCores: v })}
        />
      </Group>
      <Group legend="Icon style">
        <SelectRow<TrayStyle>
          label="Badge style"
          value={settings.trayStyle}
          options={[
            ["rounded", "Rounded"],
            ["badge", "Badge"],
            ["plain", "Plain"],
          ]}
          onChange={(v) => update({ trayStyle: v })}
        />
      </Group>
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
      <Group legend="Taskbar button">
        <Check
          label="Show temperature on the taskbar button"
          checked={settings.taskbarOn}
          onChange={(v) => update({ taskbarOn: v })}
        />
        <Check
          label="Use accent color background"
          checked={settings.taskbarAccent}
          onChange={(v) => update({ taskbarAccent: v })}
        />
      </Group>
    </>
  );
}
