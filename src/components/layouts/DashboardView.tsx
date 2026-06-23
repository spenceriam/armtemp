import { useEffect, useRef, useState } from "react";
import { SensorSnapshot } from "../../app/types";
import { formatTemp, tempColor } from "../../app/theme";
import { CardsView } from "./CardsView";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
}

const HISTORY_LEN = 48;

// Dashboard layout: a large package-temperature card with a rolling sparkline,
// followed by the per-core cards grid. History is kept in a ref-backed state
// ring buffer of real package temps.
export function DashboardView({ snap, unit }: Props) {
  const [history, setHistory] = useState<number[]>([]);
  const tickSeen = useRef<number>(-1);

  // Append each new real package reading to the rolling history.
  useEffect(() => {
    if (!snap) return;
    if (snap.tick === tickSeen.current) return; // dedupe
    tickSeen.current = snap.tick;
    if (snap.package_c !== null) {
      setHistory((h) => [...h, snap.package_c!].slice(-HISTORY_LEN));
    }
  }, [snap]);

  const pkg = snap?.package_c ?? null;
  const pkgColor = pkg !== null ? tempColor(pkg, snap?.tjmax_c ?? 100) : "var(--text-3)";
  const avg = snap?.average_c ?? null;

  return (
    <>
      <div className="card graph-card">
        <div className="graph-head">
          <div>
            <div className="graph-label">Package temperature</div>
            <div className="graph-temp" style={{ color: pkgColor }}>
              {formatTemp(pkg, unit)}
            </div>
          </div>
          <div className="graph-side">
            <div>
              Avg <span>{formatTemp(avg, unit)}</span>
            </div>
            <div>
              Power <span>{snap?.power_w != null ? `${Math.round(snap.power_w)} W` : "—"}</span>
            </div>
          </div>
        </div>
        <Sparkline history={history} color={pkgColor} tjmax={snap?.tjmax_c ?? 100} />
      </div>
      <CardsView
        cores={snap?.cores ?? []}
        tjmax={snap?.tjmax_c ?? 100}
        unit={unit}
      />
    </>
  );
}

// Minimal SVG sparkline of the package temperature history.
function Sparkline({
  history,
  color,
  tjmax,
}: {
  history: number[];
  color: string;
  tjmax: number;
}) {
  const W = 520,
    H = 92,
    lo = 30,
    hi = tjmax;
  let line = "";
  let area = "";
  if (history.length > 1) {
    const pts = history.map((t, i) => {
      const x = (i / (HISTORY_LEN - 1)) * W;
      const y = H - ((t - lo) / (hi - lo)) * H;
      return [x, y] as const;
    });
    line = pts.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
    area =
      `M${pts[0][0].toFixed(1)},${H} L` +
      pts.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" L") +
      ` L${pts[pts.length - 1][0].toFixed(1)},${H} Z`;
  }
  const gid = "armg";
  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className="sparkline">
      <defs>
        <linearGradient id={gid} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={color} stopOpacity={0.35} />
          <stop offset="1" stopColor={color} stopOpacity={0} />
        </linearGradient>
      </defs>
      {area && <path d={area} fill={`url(#${gid})`} />}
      {line && <polyline points={line} fill="none" stroke={color} strokeWidth={1.8} strokeLinejoin="round" />}
    </svg>
  );
}
