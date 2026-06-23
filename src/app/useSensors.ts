import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { SensorSnapshot } from "./types";

/// Subscribes to the live `sensor-update` event stream from the Rust backend and
/// exposes the latest real snapshot. Also pulls an initial reading on mount.
export function useSensors(): SensorSnapshot | null {
  const [snapshot, setSnapshot] = useState<SensorSnapshot | null>(null);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    (async () => {
      // Initial pull so the UI isn't blank until the first 1.5s tick.
      try {
        const initial = await invoke<SensorSnapshot | null>("get_snapshot");
        if (!cancelled && initial) setSnapshot(initial);
      } catch (e) {
        console.warn("initial snapshot failed", e);
      }
      // Subscribe to the streaming updates.
      try {
        unlisten = await listen<SensorSnapshot>("sensor-update", (e) => {
          setSnapshot(e.payload);
        });
      } catch (e) {
        console.warn("sensor event listen failed", e);
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return snapshot;
}
