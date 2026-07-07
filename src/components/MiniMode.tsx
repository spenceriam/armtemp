import { CoreReading, SensorSnapshot } from "../app/types";
import { formatTemp, tempColor } from "../app/theme";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
  colorCode: boolean;
}

function meanOf(cores: CoreReading[], pick: (c: CoreReading) => number | null): number | null {
  const vals = cores.map(pick).filter((v): v is number => v !== null);
  return vals.length ? vals.reduce((a, b) => a + b, 0) / vals.length : null;
}

/// One averaged value cell, colored by its own value the same way TempTable's
/// per-core cells are — mini mode is the normal table's math, just collapsed
/// to a single row across all cores instead of one row per core.
function AvgCell({ v, tjmax, unit, colorCode }: { v: number | null; tjmax: number; unit: "C" | "F"; colorCode: boolean }) {
  const color = v !== null && colorCode ? tempColor(v, tjmax) : undefined;
  return (
    <div className="tcell sunken mono" style={color ? { color } : undefined}>
      {formatTemp(v, unit)}
    </div>
  );
}

// Mini mode: the normal window, stripped down to the processor name and a
// single row averaging every core's current/min/max/avg temp and load — no
// Power/Tj.Max rows, no status bar. Rendered inside the same window-shell as
// normal mode (native decorations, auto-fit sizing); toggling back to full
// mode is the same "Toggle Mini Mode" menu item that entered it.
export function MiniMode({ snap, unit, colorCode }: Props) {
  const cores = snap?.cores ?? [];
  const tjmax = snap?.tjmax_c ?? 100;
  const modelStr = snap ? [snap.chip_name, snap.chip_model].filter(Boolean).join(" ") : "Detecting…";

  const avgTemp = snap?.average_c ?? null;
  const avgMin = meanOf(cores, (c) => c.min_c);
  const avgMax = meanOf(cores, (c) => c.max_c);
  const avgAvg = meanOf(cores, (c) => c.avg_c);
  const avgLoad = meanOf(cores, (c) => c.load);
  const loadStr = avgLoad !== null ? `${Math.round(avgLoad)} %` : "—";

  return (
    <div className="mini-root">
      <div className="mini-model">{modelStr}</div>
      <div className="groupbox temp-groupbox">
        <span className="groupbox-legend">Temperature Readings</span>
        <div className="temp-table">
          <div className="temp-row">
            <div className="core-cell" />
            <div className="thead">Temp</div>
            <div className="thead">Min.</div>
            <div className="thead">Max.</div>
            <div className="thead">Avg.</div>
            <div className="thead">Load</div>
          </div>
          <div className="temp-row">
            <div className="core-cell">All cores:</div>
            <AvgCell v={avgTemp} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <AvgCell v={avgMin} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <AvgCell v={avgMax} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <AvgCell v={avgAvg} tjmax={tjmax} unit={unit} colorCode={colorCode} />
            <div className="tcell sunken mono">{loadStr}</div>
          </div>
        </div>
      </div>
    </div>
  );
}
