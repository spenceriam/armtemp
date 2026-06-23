import { CoreReading } from "../../app/types";
import { formatTemp, tempColor } from "../../app/theme";

interface Props {
  cores: CoreReading[];
  tjmax: number;
  unit: "C" | "F";
}

// Classic per-core table — the design's default layout. Core temperature rows
// show "—" honestly where the firmware exposes no per-core thermal (the common
// Snapdragon X case); load is always real.
export function ClassicTable({ cores, tjmax, unit }: Props) {
  // Group cores by P/E kind for the section headers (matches the design rows).
  const groups: { label: string; rows: CoreReading[] }[] = [];
  let current: { label: string; rows: CoreReading[] } | null = null;
  for (const c of cores) {
    const label = c.kind === "efficiency" ? "Efficiency cores" : "Performance cores";
    if (!current || current.label !== label) {
      current = { label, rows: [] };
      groups.push(current);
    }
    current.rows.push(c);
  }

  return (
    <div className="card table-card">
      <div className="table-head">
        <div>Core</div>
        <div className="ta">Temp.</div>
        <div className="ta">Low</div>
        <div className="ta">High</div>
        <div className="ta">Load</div>
      </div>
      {groups.map((g) => (
        <div key={g.label}>
          <div className="table-group">{g.label}</div>
          {g.rows.map((c) => (
            <CoreRow key={c.index} c={c} tjmax={tjmax} unit={unit} />
          ))}
        </div>
      ))}
    </div>
  );
}

function CoreRow({
  c,
  tjmax,
  unit,
}: {
  c: CoreReading;
  tjmax: number;
  unit: "C" | "F";
}) {
  const hasTemp = c.temp_c !== null;
  const color = hasTemp ? tempColor(c.temp_c!, tjmax) : "var(--text-3)";
  const loadPct = c.load !== null ? Math.round(c.load) : 0;
  const loadStr = c.load !== null ? `${Math.round(c.load)} %` : "—";
  return (
    <div className="table-row">
      <div className="core-name">
        <span className="dot" style={{ background: color }} />
        <span>
          #{c.index} <span className="kind-tag">{c.kind === "efficiency" ? "E" : "P"}</span>
        </span>
      </div>
      <div className="ta mono temp-val" style={{ color }}>
        {formatTemp(c.temp_c, unit)}
      </div>
      <div className="ta mono dim">{formatTemp(c.min_c, unit)}</div>
      <div className="ta mono dim">{formatTemp(c.max_c, unit)}</div>
      <div className="load-cell">
        <div className="load-bar">
          <div
            className="load-fill"
            style={{ width: `${loadPct}%`, background: color }}
          />
        </div>
        <span className="mono load-str">{loadStr}</span>
      </div>
    </div>
  );
}
