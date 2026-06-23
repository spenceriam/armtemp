import { getCurrentWindow } from "@tauri-apps/api/window";
import { AppIcon } from "./AppIcon";

interface Props {
  version: string;
  onOpenSettings: () => void;
  unitLabel: string;
  onToggleUnit: () => void;
}

// Frameless window titlebar. The drag region uses Tauri's data-tauri-drag-region.
// Window controls call the real window APIs (minimize/close). Close is
// intercepted in Rust for close-to-tray behavior.
export function TitleBar({ version, onOpenSettings, unitLabel, onToggleUnit }: Props) {
  const win = getCurrentWindow();
  return (
    <div className="titlebar">
      <div className="titlebar-drag" data-tauri-drag-region>
        <div className="titlebar-logo">
          <AppIcon size={12} />
        </div>
        <div className="titlebar-title">
          ARMTEMP <span className="titlebar-version">{version}</span>
        </div>
        <div className="titlebar-spacer" />
      </div>

      {/* Menu bar (Options / Tools / Help) + unit toggle, lifted from the design. */}
      <div className="menubar">
        <button className="menu-item" onClick={onOpenSettings}>
          Options
        </button>
        <button
          className="menu-item"
          onClick={() => onOpenSettings()}
          title="Notification area settings"
        >
          Tools
        </button>
        <button
          className="menu-item"
          onClick={() => onOpenSettings()}
          title="About"
        >
          Help
        </button>
        <div className="menubar-spacer" />
        <button className="unit-pill" onClick={onToggleUnit} title="Toggle °C / °F">
          {unitLabel}
        </button>
      </div>

      <div className="window-controls">
        <button className="win-btn" onClick={() => win.minimize()} title="Minimize">
          <div className="ico-min" />
        </button>
        <button className="win-btn" onClick={() => win.close()} title="Close to tray">
          ✕
        </button>
      </div>
    </div>
  );
}
