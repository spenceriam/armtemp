import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettings } from "./app/useSettings";
import { useSensors } from "./app/useSensors";
import type { AppSettings } from "./app/types";
import { themeVars, ACCENT } from "./app/theme";
import { TitleBar } from "./components/TitleBar";
import { ProcessorInfo } from "./components/ProcessorInfo";
import { TempTable } from "./components/TempTable";
import { StatusBar } from "./components/StatusBar";
import { SettingsDialog } from "./components/SettingsDialog";
import { MiniMode } from "./components/MiniMode";
import { CardsView } from "./components/layouts/CardsView";
import { DashboardView } from "./components/layouts/DashboardView";

export default function App() {
  const { settings, update, loaded } = useSettings();
  const snap = useSensors();
  const [mini, setMini] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Listen for tray-menu-driven events (open-settings / toggle-mini / refresh
  // / set-tray-mode / set-unit). The mode/unit events come from the tray's
  // submenu quick-toggles and update settings (which persist + sync back to Rust).
  useEffect(() => {
    const unlistens: UnlistenFn[] = [];
    listen("open-settings", () => setSettingsOpen(true)).then((u) => unlistens.push(u));
    listen("toggle-mini", () => setMini((m) => !m)).then((u) => unlistens.push(u));
    listen("refresh-sensors", () => invoke("refresh_now").catch(() => {})).then((u) =>
      unlistens.push(u)
    );
    listen<string>("set-tray-mode", (e) => {
      update({ trayMode: e.payload as AppSettings["trayMode"] });
    }).then((u) => unlistens.push(u));
    listen<string>("set-unit", (e) => {
      update({ tempUnit: e.payload as "C" | "F" });
    }).then((u) => unlistens.push(u));
    return () => unlistens.forEach((u) => u());
  }, [update]);

  const unit = settings.tempUnit;
  const cores = snap?.cores ?? [];
  const tjmax = snap?.tjmax_c ?? 100;

  const rootStyle = themeVars(settings.theme === "system" ? "dark" : settings.theme, ACCENT);

  if (!loaded) {
    return <div className="loading" style={rootStyle}>Loading…</div>;
  }

  // Mini-mode: render only the compact widget.
  if (mini) {
    return (
      <div className="app-root mini-wrap" style={rootStyle}>
        <MiniMode
          snap={snap}
          unit={unit}
          onExpand={() => setMini(false)}
          onClose={() => getCurrentWindow().close()}
        />
      </div>
    );
  }

  return (
    <div className="app-root" style={rootStyle}>
      <div className="window-shell" style={{ transform: `scale(${settings.zoom / 100})`, transformOrigin: "top left" }}>
        <TitleBar version="0.1.0" onOpenSettings={() => setSettingsOpen(true)} />

        <div className="window-body">
          <ProcessorInfo snap={snap} />

          {/* Layout switch — Classic (default) is the CoreTemp table. */}
          {settings.uiStyle === "classic" && (
            <TempTable cores={cores} tjmax={tjmax} unit={unit} colorCode={settings.colorCodeTemps} />
          )}
          {settings.uiStyle === "cards" && <CardsView cores={cores} tjmax={tjmax} unit={unit} />}
          {settings.uiStyle === "dashboard" && <DashboardView snap={snap} unit={unit} />}
        </div>

        {settings.statusBarOn && <StatusBar snap={snap} unit={unit} />}
      </div>

      {settingsOpen && (
        <SettingsDialog
          settings={settings}
          update={update}
          tjmax={tjmax}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}
