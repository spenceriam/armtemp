import { SensorSnapshot } from "../app/types";

interface Props {
  snap: SensorSnapshot | null;
}

// The dense Processor Information block — a bordered 2-column field/value grid
// matching CoreTemp's layout exactly. All values are REAL; VID shows "—"
// honestly (no userspace surface for voltage on Snapdragon X).
export function ProcessorInfo({ snap }: Props) {
  const freqStr = (() => {
    if (!snap) return "—";
    const cur = snap.clock_mhz ?? 0;
    return `${(cur / 1000).toFixed(2)} GHz`;
  })();
  const maxFreqStr = snap?.max_clock_mhz ? `${(snap.max_clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const busStr = snap?.bus_speed_mhz != null ? `${snap.bus_speed_mhz} MHz` : "—";
  const powerStr = snap?.power_w != null ? `${Math.round(snap.power_w)} W` : "—";

  const avgLoad = (() => {
    if (!snap) return null;
    const loads = snap.cores.map((c) => c.load).filter((l): l is number => l !== null);
    if (!loads.length) return null;
    return Math.round(loads.reduce((a, b) => a + b, 0) / loads.length);
  })();

  return (
    <div className="proc-section">
      <div className="section-label">Processor Information</div>
      <div className="card proc-card">
        <Field label="Processor" value={snap?.chip_name ?? "Detecting…"} full />
        <Field label="Platform" value="ARM64 · Oryon" />
        <Field label="Vendor ID" value="Qualcomm Technologies Inc" />
        <Field label="CPUID" value={snap?.chip_model ?? "—"} />
        <Field label="Cores" value={snap ? `${snap.cores.length} (Performance)` : "—"} />
        <Field label="Threads" value={snap ? String(snap.cores.length) : "—"} />
        <Field label="Frequency" value={freqStr} />
        <Field label="Max" value={maxFreqStr} />
        <Field label="Bus speed" value={busStr} />
        <Field label="Tj. Max" value={snap ? `${snap.tjmax_c}°C` : "—"} />
        <Field label="VID" value="—" />
        <Field label="Load" value={avgLoad !== null ? `${avgLoad} %` : "—"} />
        <Field label="Power" value={powerStr} />
      </div>
    </div>
  );
}

function Field({ label, value, full }: { label: string; value: string; full?: boolean }) {
  return (
    <div className={`proc-field ${full ? "full" : ""}`}>
      <span className="proc-field-label">{label}</span>
      <span className="proc-field-value">{value}</span>
    </div>
  );
}
