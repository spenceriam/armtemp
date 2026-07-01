import { SensorSnapshot } from "../app/types";
import { SensorStatus } from "../app/useSensors";
import { ChipBadge, tierFromName } from "./ChipBadge";

interface Props {
  snap: SensorSnapshot | null;
  status: SensorStatus;
}

// The Core Temp "Processor Information" group box: Select CPU combo + a dense
// field/value grid with sunken read-only fields. All values are REAL. VID and
// Revision (permanently unavailable on Snapdragon X — no userspace surface)
// are intentionally omitted in favor of fields with live/spec data: Boost
// (profile boost clock) and Throttle (live ACPI passive-limit status).
export function ProcessorInfo({ snap, status }: Props) {
  const processorStr = snap?.chip_name ?? (status === "error" ? "Sensor unavailable" : "Detecting…");
  const modelStr = snap ? [snap.chip_name, snap.chip_model].filter(Boolean).join(" ") : processorStr;
  const freqStr = snap?.clock_mhz != null ? `${(snap.clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const boostStr = snap?.max_clock_mhz ? `${(snap.max_clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const lithographyStr = snap?.lithography || "—";
  const tdpStr = snap?.tdp_w != null ? `${snap.tdp_w} W` : "—";
  // Live thermal-throttle indicator: any valid ACPI zone reporting an active
  // passive limit (< 100 %) means the firmware is throttling right now.
  const throttled = snap ? snap.zones.some((z) => z.throttled) : null;

  return (
    <div className="proc-section">
      <div className="select-cpu">
        <ChipBadge tier={tierFromName(snap?.chip_name)} size={24} />
        <label>Select CPU:</label>
        <select disabled={!snap}>
          <option>#0 ({snap?.chip_name ?? "…"})</option>
        </select>
        <span className="cpu-count sunken">{snap ? snap.cores.length : "—"}</span>
        <span className="cpu-count-label">Core(s)</span>
        <span className="cpu-count sunken">{snap ? snap.cores.length : "—"}</span>
        <span className="cpu-count-label">Thread(s)</span>
      </div>
      <div className="groupbox">
        <span className="groupbox-legend">Processor Information</span>
        <div className="proc-grid">
          <Field label="Model" value={modelStr} full />
          <Field label="Platform" value={snap?.platform ?? "—"} full />
          <Field label="Frequency" value={freqStr} full />
          <Field label="Boost" value={boostStr} />
          <Field label="Lithography" value={lithographyStr} />
          <Field
            label="Throttle"
            value={throttled === null ? "—" : throttled ? "Yes" : "No"}
            valueColor={throttled ? "#e0473a" : undefined}
          />
          <Field label="TDP" value={tdpStr} />
          <Field label="CPUID" value={snap?.chip_model ?? "—"} full />
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  full,
  valueColor,
}: {
  label: string;
  value: string;
  full?: boolean;
  valueColor?: string;
}) {
  return (
    <>
      <span className="proc-field-label">{label}:</span>
      <span
        className={`proc-field-value sunken ${full ? "full" : ""}`}
        style={valueColor ? { color: valueColor } : undefined}
      >
        {value}
      </span>
    </>
  );
}
