import { CoreReading } from "../app/types";
import { formatLoad, formatTemp, tempColor } from "../app/theme";

interface Props {
  cores: CoreReading[];
  tjmax: number;
  unit: "C" | "F";
  colorCode: boolean;
  cpuTemp: number | null;
  cpuMin: number | null;
  cpuMax: number | null;
  cpuAvg: number | null;
}

// The Core Temp "Temperature Readings" group box: one CPU Temp row (the
// single honest CPU temperature — Snapdragon X exposes no per-core thermal
// sensor) — live current (unlabeled first column, matches the tray icon)
// plus session Min/Max/Avg — and per-core Load rows (genuinely per-core).
// Tj.Max is a spec constant, shown in Processor Information instead. No new
// colors are introduced: the CPU Temp row uses the existing temperature
// color scale; load cells stay plain text.
export function TempTable({ cores, tjmax, unit, colorCode, cpuTemp, cpuMin, cpuMax, cpuAvg }: Props) {
  return (
    <div className="groupbox temp-groupbox">
      <span className="groupbox-legend">Processor #0: Temperature Readings</span>
      <div className="temp-table">
        <div className="temp-row">
          <div />
          <div className="thead" />
          <div className="thead">Min.</div>
          <div className="thead">Max.</div>
          <div className="thead">Avg.</div>
        </div>
        <div className="temp-row">
          <div className="core-cell">CPU Temp:</div>
          <TempCell v={cpuTemp} tjmax={tjmax} unit={unit} colorCode={colorCode} />
          <TempCell v={cpuMin} tjmax={tjmax} unit={unit} colorCode={colorCode} />
          <TempCell v={cpuMax} tjmax={tjmax} unit={unit} colorCode={colorCode} />
          <TempCell v={cpuAvg} tjmax={tjmax} unit={unit} colorCode={colorCode} />
        </div>
        <div className="temp-divider" />
        <div className="temp-row">
          <div />
          <div className="thead">Load</div>
          <div className="thead">Min.</div>
          <div className="thead">Max.</div>
          <div className="thead">Avg.</div>
        </div>
        {cores.length === 0 && (
          <div className="temp-empty">Waiting for sensor data…</div>
        )}
        {cores.map((c) => (
          <CoreRow key={c.index} c={c} />
        ))}
      </div>
    </div>
  );
}

/// One value cell: its own sunken box, text colored by ITS OWN value (real
/// Core Temp colors every temperature cell independently).
function TempCell({ v, tjmax, unit, colorCode }: { v: number | null; tjmax: number; unit: "C" | "F"; colorCode: boolean }) {
  const color = v !== null && colorCode ? tempColor(v, tjmax) : undefined;
  return (
    <div className="tcell sunken mono" style={color ? { color } : undefined}>
      {formatTemp(v, unit)}
    </div>
  );
}

/// One plain (uncolored) load cell — load has no temperature-style coloring.
function LoadCell({ v }: { v: number | null }) {
  return <div className="tcell sunken mono">{formatLoad(v)}</div>;
}

function CoreRow({ c }: { c: CoreReading }) {
  return (
    <div className="temp-row">
      <div className="core-cell">
        <span>Core #{c.index}:</span>
        <span className="kind-tag" title={c.kind_title}>
          {c.kind_label}
        </span>
      </div>
      <LoadCell v={c.load} />
      <LoadCell v={c.load_min} />
      <LoadCell v={c.load_max} />
      <LoadCell v={c.load_avg} />
    </div>
  );
}
