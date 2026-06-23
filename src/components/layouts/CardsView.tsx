import { CoreReading } from "../../app/types";
import { formatTemp, tempColor } from "../../app/theme";

interface Props {
  cores: CoreReading[];
  tjmax: number;
  unit: "C" | "F";
}

// Cards view — one tile per core (the design's "cards" uiStyle).
export function CardsView({ cores, tjmax, unit }: Props) {
  return (
    <div className="cards-grid">
      {cores.map((c) => (
        <CoreCard key={c.index} c={c} tjmax={tjmax} unit={unit} />
      ))}
    </div>
  );
}

function CoreCard({
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
  return (
    <div className="card core-card">
      <div className="core-card-head">
        <span className="mono dim">#{c.index}</span>
        <span className="kind-chip">{c.kind === "efficiency" ? "E" : "P"}</span>
      </div>
      <div className="core-card-temp" style={{ color }}>
        {formatTemp(c.temp_c, unit)}
      </div>
      <div className="load-bar">
        <div className="load-fill" style={{ width: `${loadPct}%`, background: color }} />
      </div>
      <div className="core-card-foot mono">
        <span>{c.load !== null ? `${Math.round(c.load)} %` : "—"}</span>
        <span>
          {formatTemp(c.min_c, unit)} / {formatTemp(c.max_c, unit)}
        </span>
      </div>
    </div>
  );
}
