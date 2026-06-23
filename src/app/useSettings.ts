import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import { AppSettings, DEFAULT_SETTINGS } from "./types";

const STORE_FILE = "settings.json";

/// Persistent app settings backed by the Tauri store plugin (JSON in app data).
/// All UI controls bind through this hook.
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
        if (!cancelled) {
          setStore(s);
          setSettings(merged);
          setLoaded(true);
        }
      } catch {
        if (!cancelled) setLoaded(true);
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
        // Fire and forget persistence + backend sync.
        (async () => {
          try {
            if (store) {
              for (const [k, v] of Object.entries(patch)) {
                await store.set(k, v);
              }
              await store.save();
            }
            // Mirror the subset the Rust tray/overheat logic cares about.
            await invoke("update_settings", {
              settings: {
                tempUnit: next.tempUnit,
                trayMode: next.trayMode,
                trayStyle: next.trayStyle,
                overheatOn: next.overheatOn,
                overheatThreshold: next.overheatThreshold,
              },
            });
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
