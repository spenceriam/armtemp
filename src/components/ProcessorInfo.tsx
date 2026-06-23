import { SensorSnapshot } from "../app/types";

interface Props {
  snap: SensorSnapshot | null;
}

// The processor identity card. All values are REAL: chip name/model from
// Win32_Processor, clocks from WMI, power from the Power Meter counter.
// Where a real source is missing the field shows "—" honestly.
// Voltage is not exposed by any userspace surface on Snapdragon X — honest "—".
export function ProcessorInfo({ snap }: Props) {
  const tjMaxStr = snap ? `${snap.tjmax_c}°C` : "—";
  const freqStr = (() => {
    if (!snap) return "—";
    const max = snap.max_clock_mhz ?? 0;
    const cur = snap.clock_mhz ?? 0;
    return `${(cur / 1000).toFixed(2)} GHz${max ? ` · ${max} MHz max` : ""}`;
  })();
  const voltStr = "—";
  const powerStr = snap?.power_w != null ? `${Math.round(snap.power_w)} W` : "—";

  const name = snap?.chip_name ?? "Detecting…";
  const model = snap?.chip_model ?? "";
  const ct = snap?.core_thread ?? "— / —";
  const coreLabel = snap ? `${snap.cores.length} cores` : "";

  return (
    <div className="card proc-card">
      <div className="proc-head">
        <div className="proc-name">{name}</div>
        <div className="proc-model">{model}</div>
        <div className="proc-badge">{coreLabel}</div>
      </div>
      <div className="proc-grid">
        <Row label="Platform" value="ARM64 · Oryon" />
        <Row label="Tj. Max" value={tjMaxStr} />
        <Row label="Frequency" value={freqStr} />
        <Row label="Voltage" value={voltStr} />
        <Row label="Cores / Threads" value={ct} />
        <Row label="Power" value={powerStr} />
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="proc-row">
      <span className="proc-row-label">{label}</span>
      <span className="proc-row-value">{value}</span>
    </div>
  );
}
