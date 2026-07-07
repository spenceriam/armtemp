import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isEnabled as autostartIsEnabled, enable as autostartEnable, disable as autostartDisable } from "@tauri-apps/plugin-autostart";
import { AppSettings, DEFAULT_SETTINGS } from "./types";

const STORE_FILE = "settings.json";

/// Mirror the fields the Rust backend actually reads (tray, poll loop,
/// overheat, taskbar overlay, close/minimize behavior). Keep in sync with
/// `AppSettings` in src-tauri/src/lib.rs.
function backendSettings(s: AppSettings) {
  return {
    tempUnit: s.tempUnit,
    trayStyle: s.trayStyle,
    overheatOn: s.overheatOn,
    overheatThreshold: s.overheatThreshold,
    overheatAction: s.overheatAction,
    closeToTray: s.closeToTray,
    hideWhenMinimized: s.hideWhenMinimized,
    pollingIntervalMs: s.pollingIntervalMs,
    trayOn: s.trayOn,
    trayTooltipAllCores: s.trayTooltipAllCores,
    taskbarOn: s.taskbarOn,
    taskbarAccent: s.taskbarAccent,
    theme: s.theme,
  };
}

/// Persistent app settings backed by the Tauri store plugin (JSON in app data).
/// All UI controls bind through this hook. A few settings (autostart, always-
/// on-top) are pure window/OS toggles applied directly here rather than
/// mirrored to the Rust backend.
export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [store, setStore] = useState<Store | null>(null);
  const [loaded, setLoaded] = useState(false);

  // Load once on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const s = await Store.load(STORE_FILE);
        const entries = await s.entries();
        const merged: AppSettings = { ...DEFAULT_SETTINGS };
        for (const [k, v] of entries) {
          (merged as unknown as Record<string, unknown>)[k] = v;
        }

        // Reconcile startWithWindows against the OS's actual autostart state —
        // the user may have removed it from Windows Settings directly.
        try {
          const actuallyEnabled = await autostartIsEnabled();
          if (actuallyEnabled !== merged.startWithWindows) {
            merged.startWithWindows = actuallyEnabled;
            await s.set("startWithWindows", actuallyEnabled);
          }
        } catch (e) {
          console.warn("autostart reconcile failed", e);
        }

        // Apply always-on-top from the persisted setting (window defaults off).
        if (merged.alwaysOnTop) {
          try {
            await getCurrentWindow().setAlwaysOnTop(true);
          } catch (e) {
            console.warn("setAlwaysOnTop failed", e);
          }
        }

        if (!cancelled) {
          setStore(s);
          setSettings(merged);
          setLoaded(true);
          // Push the initial backend-relevant subset so Rust isn't stuck on
          // AppSettings::default() until the user changes something.
          invoke("update_settings", { settings: backendSettings(merged) }).catch(() => {});
        }
      } catch {
        if (!cancelled) {
          setLoaded(true);
          // Store unreadable/corrupt: still sync defaults so the Rust mirror
          // (tray, close-to-tray, overheat) is never left stale.
          invoke("update_settings", { settings: backendSettings(DEFAULT_SETTINGS) }).catch(() => {});
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Persist + notify the Rust backend (for tray/overheat) on every change.
  const update = useCallback(
    async (patch: Partial<AppSettings>) => {
      setSettings((prev) => {
        const next = { ...prev, ...patch };
        // Fire and forget persistence + backend sync + OS-level side effects.
        (async () => {
          try {
            if (store) {
              for (const [k, v] of Object.entries(patch)) {
                await store.set(k, v);
              }
              await store.save();
            }
            await invoke("update_settings", { settings: backendSettings(next) });

            if ("startWithWindows" in patch) {
              if (patch.startWithWindows) await autostartEnable();
              else await autostartDisable();
            }
            if ("alwaysOnTop" in patch) {
              await getCurrentWindow().setAlwaysOnTop(!!patch.alwaysOnTop);
            }
          } catch (e) {
            console.warn("settings persist failed", e);
          }
        })();
        return next;
      });
    },
    [store]
  );

  return { settings, update, loaded };
}
