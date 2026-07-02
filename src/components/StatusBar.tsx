import { SensorSnapshot } from "../app/types";
import { formatTemp } from "../app/theme";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
}

// The sunken status bar footer: Avg Core Temp / Low / High. All real values.
// (No single "CPU Temp" — Snapdragon X exposes zone temps mapped to cores, not
// one authoritative package sensor, so a lone number would misrepresent which
// core it came from.)
export function StatusBar({ snap, unit }: Props) {
  const avg = snap?.average_c ?? null;
  const lows = snap?.cores.map((c) => c.min_c).filter((t): t is number => t !== null) ?? [];
  const highs = snap?.cores.map((c) => c.max_c).filter((t): t is number => t !== null) ?? [];
  const low = lows.length ? Math.min(...lows) : null;
  const high = highs.length ? Math.max(...highs) : null;
  return (
    <div className="status-bar">
      <span className="sb-item">
        <span className="sb-label">Avg Core Temp:</span> {formatTemp(avg, unit)}
      </span>
      <span className="sb-sep" />
      <span className="sb-item">
        <span className="sb-label">Low:</span> {formatTemp(low, unit)}
      </span>
      <span className="sb-sep" />
      <span className="sb-item">
        <span className="sb-label">High:</span> {formatTemp(high, unit)}
      </span>
      <span className="sb-grip" />
    </div>
  );
}
