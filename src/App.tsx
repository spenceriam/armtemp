import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useSettings } from "./app/useSettings";
import { useSensors } from "./app/useSensors";
import type { AppSettings } from "./app/types";
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
  // / set-tray-mode / set-unit). The mode/unit events come from the tray's
  // submenu quick-toggles and update settings (which persist + sync back to Rust).
  useEffect(() => {
    const unlistens: UnlistenFn[] = [];
    listen("open-settings", () => setSettingsOpen(true)).then((u) => unlistens.push(u));
    listen("open-overheat", () => setOverheatOpen(true)).then((u) => unlistens.push(u));
    listen("open-about", () => setAboutOpen(true)).then((u) => unlistens.push(u));
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

  // Mini-mode: drop native decorations + shrink to a compact always-on-top
  // box (matches Core Temp's mini mode). On exit the auto-fit effect below
  // restores the window to its content size.
  useEffect(() => {
    const win = getCurrentWindow();
    (async () => {
      try {
        if (mini) {
          await win.setDecorations(false);
          await win.setSize(new LogicalSize(280, 130));
          await win.setAlwaysOnTop(true);
        } else {
          await win.setDecorations(true);
          await win.setAlwaysOnTop(settings.alwaysOnTop);
        }
      } catch (e) {
        console.warn("mini-mode window ops failed", e);
      }
    })();
    // deliberately not depending on settings.alwaysOnTop — only re-run on mini toggle
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mini]);

  // Content-fit window (Core Temp behavior): size the OS window to the
  // shell's rendered rect. The shell has a fixed 564px layout width and
  // natural height (styles.css), so its rect never depends on the window
  // size — no resize feedback loop — and the zoom transform is included in
  // the measured rect automatically.
  const shellRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (mini) return; // mini-mode manages its own window size
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
      <div ref={shellRef} className="window-shell" style={{ transform: `scale(${settings.zoom / 100})`, transformOrigin: "top left" }}>
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
          <ProcessorInfo snap={snap} status={sensorStatus} />

          {/* Layout switch — Classic (default) is the CoreTemp table. */}
          {settings.uiStyle === "classic" && (
            <TempTable
              cores={cores}
              tjmax={tjmax}
              unit={unit}
              colorCode={settings.colorCodeTemps}
              powerW={snap?.power_w ?? null}
            />
          )}
          {settings.uiStyle === "cards" && <CardsView cores={cores} tjmax={tjmax} unit={unit} />}
          {settings.uiStyle === "dashboard" && <DashboardView snap={snap} unit={unit} />}
        </div>

        {settings.statusBarOn && <StatusBar snap={snap} unit={unit} />}
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
