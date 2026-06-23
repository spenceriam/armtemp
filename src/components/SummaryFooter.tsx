import { SensorSnapshot } from "../app/types";
import { formatTemp, tempColor } from "../app/theme";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
  onToggleMini: () => void;
  onOpenSettings: () => void;
}

export function SummaryFooter({ snap, unit, onToggleMini, onOpenSettings }: Props) {
  const pkg = snap?.package_c ?? null;
  const pkgColor = pkg !== null ? tempColor(pkg, snap?.tjmax_c ?? 100) : "var(--text-3)";
  const avg = snap?.average_c ?? null;
  return (
    <div className="card summary-footer">
      <div className="summary-item">
        <div className="summary-label">Package</div>
        <div className="summary-val" style={{ color: pkgColor }}>
          {formatTemp(pkg, unit)}
        </div>
      </div>
      <div className="summary-sep" />
      <div className="summary-item">
        <div className="summary-label">Average</div>
        <div className="summary-val">{formatTemp(avg, unit)}</div>
      </div>
      <div className="summary-spacer" />
      <button className="btn-ghost" onClick={onToggleMini}>
        Mini-mode
      </button>
      <button className="btn-accent" onClick={onOpenSettings}>
        Settings
      </button>
    </div>
  );
}
