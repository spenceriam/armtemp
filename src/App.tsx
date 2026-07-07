import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useSettings } from "./app/useSettings";
import { useSensors } from "./app/useSensors";
import { themeVars, ACCENT } from "./app/theme";
import { MenuBar } from "./components/MenuBar";
import { ProcessorInfo } from "./components/ProcessorInfo";
import { TempTable } from "./components/TempTable";
import { StatusBar } from "./components/StatusBar";
import { SettingsDialog } from "./components/SettingsDialog";
import { OverheatDialog } from "./components/OverheatDialog";
import { AboutDialog } from "./components/AboutDialog";
import { MiniMode } from "./components/MiniMode";
import { CardsView } from "./components/layouts/CardsView";
import { DashboardView } from "./components/layouts/DashboardView";

export default function App() {
  const { settings, update, loaded } = useSettings();
  const { snap, status: sensorStatus } = useSensors();
  const [mini, setMini] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [overheatOpen, setOverheatOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);

  // Listen for tray-menu-driven events (open-settings / toggle-mini / refresh
  // / set-unit). The unit event comes from the tray's submenu quick-toggle
  // and updates settings (which persist + sync back to Rust).
  useEffect(() => {
    const unlistens: UnlistenFn[] = [];
    listen("open-settings", () => setSettingsOpen(true)).then((u) => unlistens.push(u));
    listen("open-overheat", () => setOverheatOpen(true)).then((u) => unlistens.push(u));
    listen("open-about", () => setAboutOpen(true)).then((u) => unlistens.push(u));
    listen("toggle-mini", () => setMini((m) => !m)).then((u) => unlistens.push(u));
    listen("refresh-sensors", () => invoke("refresh_now").catch(() => {})).then((u) =>
      unlistens.push(u)
    );
    listen<string>("set-unit", (e) => {
      update({ tempUnit: e.payload as "C" | "F" });
    }).then((u) => unlistens.push(u));
    return () => unlistens.forEach((u) => u());
  }, [update]);

  // Content-fit window (Core Temp behavior): size the OS window to the
  // shell's rendered rect. The shell has a fixed 564px layout width and
  // natural height (styles.css), so its rect never depends on the window
  // size — no resize feedback loop. Mini mode uses the same shell/effect —
  // it's just a smaller rendered rect, so the window shrinks to match.
  const shellRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = shellRef.current;
    if (!el) return;
    let last = { w: 0, h: 0 };
    const fit = () => {
      const r = el.getBoundingClientRect();
      const w = Math.ceil(r.width);
      const h = Math.ceil(r.height);
      if (Math.abs(w - last.w) <= 1 && Math.abs(h - last.h) <= 1) return;
      last = { w, h };
      getCurrentWindow()
        .setSize(new LogicalSize(w, h))
        .catch((e) => console.warn("auto-fit failed", e));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
    // `loaded` is required here even though it's not read directly: `.window-shell`
    // doesn't exist in the DOM until the `!loaded` early-return above stops firing,
    // so the observer must be (re-)attached once that transition happens.
  }, [mini, loaded]);

  const unit = settings.tempUnit;
  const cores = snap?.cores ?? [];
  const tjmax = snap?.tjmax_c ?? 100;

  const rootStyle = themeVars(settings.theme === "system" ? "dark" : settings.theme, ACCENT);

  if (!loaded) {
    return <div className="loading" style={rootStyle}>Loading…</div>;
  }

  return (
    <div className="app-root" style={rootStyle}>
      <div ref={shellRef} className="window-shell">
        <MenuBar
          alwaysOnTop={settings.alwaysOnTop}
          onOpenSettings={() => setSettingsOpen(true)}
          onOpenOverheat={() => setOverheatOpen(true)}
          onOpenAbout={() => setAboutOpen(true)}
          onToggleMini={() => setMini((m) => !m)}
          onToggleAlwaysOnTop={() => update({ alwaysOnTop: !settings.alwaysOnTop })}
          onRefresh={() => invoke("refresh_now").catch(() => {})}
          onExit={() => invoke("exit_app").catch(() => {})}
        />

        <div className="window-body">
          {mini ? (
            <MiniMode snap={snap} unit={unit} colorCode={settings.colorCodeTemps} />
          ) : (
            <>
              <ProcessorInfo snap={snap} status={sensorStatus} />

              {/* Layout switch — Classic (default) is the CoreTemp table. */}
              {settings.uiStyle === "classic" && (
                <TempTable
                  cores={cores}
                  tjmax={tjmax}
                  unit={unit}
                  colorCode={settings.colorCodeTemps}
                  cpuTemp={snap?.package_c ?? null}
                  cpuMin={snap?.package_min_c ?? null}
                  cpuMax={snap?.package_max_c ?? null}
                  cpuAvg={snap?.package_avg_c ?? null}
                />
              )}
              {settings.uiStyle === "cards" && <CardsView cores={cores} />}
              {settings.uiStyle === "dashboard" && <DashboardView snap={snap} unit={unit} />}
            </>
          )}
        </div>

        {!mini && settings.statusBarOn && <StatusBar snap={snap} unit={unit} />}
      </div>

      {settingsOpen && (
        <SettingsDialog settings={settings} update={update} onClose={() => setSettingsOpen(false)} />
      )}
      {overheatOpen && (
        <OverheatDialog
          settings={settings}
          update={update}
          tjmax={tjmax}
          onClose={() => setOverheatOpen(false)}
        />
      )}
      {aboutOpen && <AboutDialog onClose={() => setAboutOpen(false)} />}
    </div>
  );
}
