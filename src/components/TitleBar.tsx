import { getCurrentWindow } from "@tauri-apps/api/window";
import { AppIcon } from "./AppIcon";

interface Props {
  version: string;
  onOpenSettings: () => void;
}

// Frameless window titlebar. Unit (°C/°F) is NOT here — it lives in Settings →
// General. The menu bar is just Tools / Options / Help.
export function TitleBar({ version, onOpenSettings }: Props) {
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

      {/* Menu bar — Tools / Options / Help. No unit toggle. */}
      <div className="menubar">
        <button className="menu-item" onClick={onOpenSettings} title="Tools">
          Tools
        </button>
        <button className="menu-item" onClick={onOpenSettings} title="Options / Settings">
          Options
        </button>
        <button className="menu-item" onClick={onOpenSettings} title="About">
          Help
        </button>
        <div className="menubar-spacer" />
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
