import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { SensorSnapshot } from "./types";

export type SensorStatus = "loading" | "ready" | "error";

const FIRST_SNAPSHOT_TIMEOUT_MS = 6000;

/// Subscribes to the live `sensor-update` event stream from the Rust backend and
/// exposes the latest real snapshot plus an honest status: "loading" until the
/// first reading arrives, "error" if none arrives within a few seconds (never a
/// fabricated chip name), "ready" once real data is flowing.
export function useSensors(): { snap: SensorSnapshot | null; status: SensorStatus } {
  const [snapshot, setSnapshot] = useState<SensorSnapshot | null>(null);
  const [status, setStatus] = useState<SensorStatus>("loading");

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    let gotAny = false;

    const timeout = setTimeout(() => {
      if (!cancelled && !gotAny) setStatus("error");
    }, FIRST_SNAPSHOT_TIMEOUT_MS);

    const onSnapshot = (s: SensorSnapshot) => {
      gotAny = true;
      setSnapshot(s);
      setStatus("ready");
    };

    (async () => {
      // Initial pull so the UI isn't blank until the first 1.5s tick.
      try {
        const initial = await invoke<SensorSnapshot | null>("get_snapshot");
        if (!cancelled && initial) onSnapshot(initial);
      } catch (e) {
        console.warn("initial snapshot failed", e);
      }
      // Subscribe to the streaming updates.
      try {
        unlisten = await listen<SensorSnapshot>("sensor-update", (e) => {
          if (!cancelled) onSnapshot(e.payload);
        });
      } catch (e) {
        console.warn("sensor event listen failed", e);
        if (!cancelled && !gotAny) setStatus("error");
      }
    })();

    return () => {
      cancelled = true;
      clearTimeout(timeout);
      unlisten?.();
    };
  }, []);

  return { snap: snapshot, status };
}
