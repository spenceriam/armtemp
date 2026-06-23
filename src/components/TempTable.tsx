import { CoreReading } from "../app/types";
import { formatTemp, tempColor } from "../app/theme";

interface Props {
  cores: CoreReading[];
  tjmax: number;
  unit: "C" | "F";
  colorCode: boolean;
}

// The CoreTemp-style per-core temperature table:
//   Core # | Temp ● | Low | High | Load
// Per-core temps are REAL (zone-mapped: Core #0 = hottest zone).
export function TempTable({ cores, tjmax, unit, colorCode }: Props) {
  return (
    <div className="temp-section">
      <div className="section-label">Temperature Readings</div>
      <div className="card temp-table">
        <div className="temp-head">
          <div>Core</div>
          <div className="ta">Temp.</div>
          <div className="ta">Low</div>
          <div className="ta">High</div>
          <div className="load-col">Load</div>
        </div>
        {cores.length === 0 && (
          <div className="temp-empty">No sensor data — reading from on-die thermal sensors…</div>
        )}
        {cores.map((c) => (
          <CoreRow key={c.index} c={c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
        ))}
      </div>
      {colorCode && (
        <div className="temp-legend">
          <span className="legend-item"><span className="dot" style={{ background: "#2fa45a" }} /> ≤45%</span>
          <span className="legend-item"><span className="dot" style={{ background: "#d8a51a" }} /> ≤64%</span>
          <span className="legend-item"><span className="dot" style={{ background: "#e07a2b" }} /> ≤82%</span>
          <span className="legend-item"><span className="dot" style={{ background: "#e0473a" }} /> &gt;82%</span>
        </div>
      )}
    </div>
  );
}

function CoreRow({
  c,
  tjmax,
  unit,
  colorCode,
}: {
  c: CoreReading;
  tjmax: number;
  unit: "C" | "F";
  colorCode: boolean;
}) {
  const hasTemp = c.temp_c !== null;
  const color = hasTemp && colorCode ? tempColor(c.temp_c!, tjmax) : "var(--text)";
  const loadPct = c.load !== null ? Math.round(c.load) : 0;
  const loadStr = c.load !== null ? `${Math.round(c.load)} %` : "—";
  return (
    <div className="temp-row">
      <div className="core-cell">
        <span className={`dot ${hasTemp ? "" : "empty"}`} style={{ background: hasTemp ? color : "var(--text-3)" }} />
        <span>Core #{c.index}</span>
        <span className="kind-tag">{c.kind === "efficiency" ? "E" : "P"}</span>
      </div>
      <div className="ta mono temp-val" style={{ color: hasTemp ? color : "var(--text-3)" }}>
        {formatTemp(c.temp_c, unit)}
      </div>
      <div className="ta mono dim">{formatTemp(c.min_c, unit)}</div>
      <div className="ta mono dim">{formatTemp(c.max_c, unit)}</div>
      <div className="load-col">
        <div className="load-bar">
          <div className="load-fill" style={{ width: `${loadPct}%`, background: hasTemp && colorCode ? color : "var(--accent)" }} />
        </div>
        <span className="mono load-str">{loadStr}</span>
      </div>
    </div>
  );
}
