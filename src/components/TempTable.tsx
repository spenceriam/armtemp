import { CoreReading } from "../app/types";
import { formatTemp, tempColor } from "../app/theme";

interface Props {
  cores: CoreReading[];
  tjmax: number;
  unit: "C" | "F";
  colorCode: boolean;
  powerW: number | null;
}

// The Core Temp "Temperature Readings" group box: Tj. Max row, per-core rows
// with colored temperature TEXT (no dots/bars — the color itself carries the
// meaning), and a Power row. Per-core temps are REAL (zone-mapped: Core #0 =
// hottest zone).
export function TempTable({ cores, tjmax, unit, colorCode, powerW }: Props) {
  return (
    <div className="groupbox temp-groupbox">
      <span className="groupbox-legend">Processor #0: Temperature Readings</span>
      <div className="temp-table">
        {powerW != null && (
          <div className="temp-row">
            <div className="core-cell">Power:</div>
            <div className="tcell sunken mono">{Math.round(powerW)} W</div>
            <div />
            <div />
            <div />
            <div />
          </div>
        )}
        {/* Tj. Max row doubles as the column-header row, like real Core Temp. */}
        <div className="temp-row">
          <div className="core-cell">Tj. Max:</div>
          <div className="tcell sunken mono">{tjmax}°C</div>
          <div className="thead">Min.</div>
          <div className="thead">Max.</div>
          <div className="thead">Avg.</div>
          <div className="thead">Load</div>
        </div>
        {cores.length === 0 && (
          <div className="temp-empty">No sensor data — reading from on-die thermal sensors…</div>
        )}
        {cores.map((c) => (
          <CoreRow key={c.index} c={c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
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
  const loadStr = c.load !== null ? `${Math.round(c.load)} %` : "—";
  return (
    <div className="temp-row">
      <div className="core-cell">
        <span>Core #{c.index}:</span>
        <span className="kind-tag">{c.kind === "efficiency" ? "E" : "P"}</span>
      </div>
      <TempCell v={c.temp_c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
      <TempCell v={c.min_c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
      <TempCell v={c.max_c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
      <TempCell v={c.avg_c} tjmax={tjmax} unit={unit} colorCode={colorCode} />
      <div className="tcell sunken mono">{loadStr}</div>
    </div>
  );
}
