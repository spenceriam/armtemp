import { CoreReading } from "../../app/types";
import { formatLoad } from "../../app/theme";

interface Props {
  cores: CoreReading[];
}

// Cards view — one tile per core (the design's "cards" uiStyle). Snapdragon X
// exposes no per-core temperature sensor, so each card shows real per-core
// LOAD (the single honest CPU temperature is shown in the Dashboard header).
export function CardsView({ cores }: Props) {
  return (
    <div className="cards-grid">
      {cores.map((c) => (
        <CoreCard key={c.index} c={c} />
      ))}
    </div>
  );
}

function CoreCard({ c }: { c: CoreReading }) {
  const loadPct = c.load !== null ? Math.round(c.load) : 0;
  return (
    <div className="card core-card">
      <div className="core-card-head">
        <span className="mono dim">#{c.index}</span>
        <span className="kind-chip" title={c.kind === "efficiency" ? "Efficiency core" : "Performance core"}>
          {c.kind === "efficiency" ? "E" : "P"}
        </span>
      </div>
      <div className="core-card-temp">{formatLoad(c.load)}</div>
      <div className="load-bar">
        <div className="load-fill" style={{ width: `${loadPct}%`, background: "var(--accent)" }} />
      </div>
      <div className="core-card-foot mono">
        <span>Load</span>
        <span>
          {formatLoad(c.load_min)} / {formatLoad(c.load_max)}
        </span>
      </div>
    </div>
  );
}
