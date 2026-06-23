import { SensorSnapshot } from "../app/types";
import { formatTemp, tempColor } from "../app/theme";
import { AppIcon } from "./AppIcon";

interface Props {
  snap: SensorSnapshot | null;
  unit: "C" | "F";
  onExpand: () => void;
  onClose: () => void;
}

// The compact always-on-top readout. Package temp large, with a row of per-core
// color swatches (load-driven color when per-core temp isn't available).
export function MiniMode({ snap, unit, onExpand, onClose }: Props) {
  const pkg = snap?.package_c ?? null;
  const pkgColor = pkg !== null ? tempColor(pkg, snap?.tjmax_c ?? 100) : "var(--text-3)";
  return (
    <div className="mini-root">
      <div className="mini-head" data-tauri-drag-region>
        <div className="mini-logo">
          <AppIcon size={10} />
        </div>
        <span className="mini-title">ARMTEMP</span>
        <div className="mini-spacer" />
        <button className="mini-btn" onClick={onExpand} title="Expand">
          ▢
        </button>
        <button className="mini-btn close" onClick={onClose} title="Close">
          ✕
        </button>
      </div>
      <div className="mini-body">
        <div className="mini-temp" style={{ color: pkgColor }}>
          {formatTemp(pkg, unit)}
        </div>
        <div className="mini-swatches">
          {(snap?.cores ?? []).map((c) => {
            const color =
              c.temp_c !== null
                ? tempColor(c.temp_c, snap!.tjmax_c)
                : c.load !== null
                ? tempColor(35 + (c.load / 100) * 60, snap!.tjmax_c)
                : "var(--text-3)";
            return <div key={c.index} className="mini-swatch" style={{ background: color }} title={`#${c.index}`} />;
          })}
        </div>
      </div>
    </div>
  );
}
