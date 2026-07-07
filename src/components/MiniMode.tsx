import { CoreReading, SensorSnapshot } from "../app/types";
import { formatLoad, formatTemp, tempColor } from "../app/theme";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
  colorCode: boolean;
}

function meanOf(cores: CoreReading[], pick: (c: CoreReading) => number | null): number | null {
  const vals = cores.map(pick).filter((v): v is number => v !== null);
  return vals.length ? vals.reduce((a, b) => a + b, 0) / vals.length : null;
}

/// One temperature cell, colored by its own value (matches TempTable's CPU
/// Temp row).
function TempCell({ v, tjmax, unit, colorCode }: { v: number | null; tjmax: number; unit: "C" | "F"; colorCode: boolean }) {
  const color = v !== null && colorCode ? tempColor(v, tjmax) : undefined;
  return (
    <div className="tcell sunken mono" style={color ? { color } : undefined}>
      {formatTemp(v, unit)}
    </div>
  );
}

/// One plain (uncolored) load cell.
function LoadCell({ v }: { v: number | null }) {
  return <div className="tcell sunken mono">{formatLoad(v)}</div>;
}

// Mini mode: the normal window, stripped down to the processor name, the
// single honest CPU temperature, and an averaged per-core Load row — no
// Power/Tj.Max rows, no status bar. Rendered inside the same window-shell as
// normal mode (native decorations, auto-fit sizing); toggling back to full
// mode is the same "Toggle Mini Mode" menu item that entered it.
export function MiniMode({ snap, unit, colorCode }: Props) {
  const cores = snap?.cores ?? [];
  const tjmax = snap?.tjmax_c ?? 100;
  const modelStr = snap ? [snap.chip_name, snap.chip_model].filter(Boolean).join(" ") : "Detecting…";

  const cpuTemp = snap?.package_c ?? null;
  const cpuMin = snap?.package_min_c ?? null;
  const cpuMax = snap?.package_max_c ?? null;
  const cpuAvg = snap?.package_avg_c ?? null;

  const avgLoad = meanOf(cores, (c) => c.load);
  const avgLoadMin = meanOf(cores, (c) => c.load_min);
  const avgLoadMax = meanOf(cores, (c) => c.load_max);
  const avgLoadAvg = meanOf(cores, (c) => c.load_avg);

  return (
    <div className="mini-root">
      <div className="mini-model">{modelStr}</div>
      <div className="groupbox temp-groupbox">
        <span className="groupbox-legend">Temperature Readings</span>
        <div className="temp-table">
          <div className="temp-row">
            <div className="core-cell" />
            <div className="thead">Cur.</div>
            <div className="thead">Min.</div>
            <div className="thead">Max.</div>
            <div className="thead">Avg.</div>
          </div>
          <div className="temp-row">
            <div className="core-cell">CPU:</div>
            <TempCell v={cpuTemp} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <TempCell v={cpuMin} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <TempCell v={cpuMax} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <TempCell v={cpuAvg} tjmax={tjmax} unit={unit} colorCode={colorCode} />
          </div>
          <div className="temp-row">
            <div className="core-cell">All cores:</div>
            <LoadCell v={avgLoad} />
            <LoadCell v={avgLoadMin} />
            <LoadCell v={avgLoadMax} />
            <LoadCell v={avgLoadAvg} />
          </div>
        </div>
      </div>
    </div>
  );
}
