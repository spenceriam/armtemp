import { SensorSnapshot } from "../app/types";
import { SensorStatus } from "../app/useSensors";

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
  // "Unknown ... SKU" isn't a real model number — prefixing `chip_name` in
  // front of it reads as broken ("Snapdragon X2 Elite Unknown X2 Elite
  // SKU"). Show just the family name in that case; a real (even ambiguous,
  // e.g. "X2E-80/84-100") model still gets the full string.
  const isPlaceholderModel = !snap?.chip_model || /^unknown\b/i.test(snap.chip_model);
  const modelStr = snap ? (isPlaceholderModel ? snap.chip_name : [snap.chip_name, snap.chip_model].join(" ")) : processorStr;
  const speedStr = snap?.clock_mhz != null ? `${(snap.clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const baseStr = snap?.base_clock_mhz ? `${(snap.base_clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const boostStr = snap?.max_clock_mhz ? `${(snap.max_clock_mhz / 1000).toFixed(2)} GHz` : "—";
  const lithographyStr = snap?.lithography || "—";
  const tdpStr = snap?.tdp_w != null ? `${snap.tdp_w} W` : "—";
  // The machine's real CPUID-derived identity (registry `Identifier`, e.g.
  // "ARMv8 (64-bit) Family 8 Model 2 Revision 201") — not a repeat of the
  // marketing model string, which the Model field above already shows.
  const cpuidStr = snap?.cpu_identifier || "—";
  // Live thermal-throttle indicator: any valid ACPI zone reporting an active
  // passive limit (< 100 %) means the firmware is throttling right now.
  const throttled = snap ? snap.zones.some((z) => z.throttled) : null;

  return (
    <div className="proc-section">
      <div className="select-cpu">
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
          <Field
            label="Model"
            value={modelStr}
            full
            title={
              snap?.detection_basis && snap.detection_basis !== "exact SKU token in CPU name"
                ? `Detection: ${snap.detection_basis}`
                : undefined
            }
          />
          <Field label="Platform" value={snap?.platform ?? "—"} full />
          <Field label="Speed" value={speedStr} full />
          <Field label="Base" value={baseStr} />
          <Field label="Boost" value={boostStr} />
          <Field label="Lithography" value={lithographyStr} />
          <Field
            label="Throttle"
            value={throttled === null ? "—" : throttled ? "Yes" : "No"}
            valueColor={throttled ? "#e0473a" : undefined}
          />
          <Field label="TDP" value={tdpStr} full />
          <Field label="CPUID" value={cpuidStr} full />
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
  title,
}: {
  label: string;
  value: string;
  full?: boolean;
  valueColor?: string;
  title?: string;
}) {
  // Sunken fields visually truncate long values (e.g. CPUID's registry
  // Identifier string). Default the hover tooltip to the value itself so the
  // full text is always available on hover, with zero layout/UI change;
  // callers with something more useful to say (e.g. Model's detection basis)
  // pass an explicit `title` that takes precedence.
  return (
    <>
      <span className="proc-field-label">{label}:</span>
      <span
        className={`proc-field-value sunken ${full ? "full" : ""}`}
        style={valueColor ? { color: valueColor } : undefined}
        title={title ?? value}
      >
        {value}
      </span>
    </>
  );
}
