import { ChipsList } from "./ChipsList";

interface Props {
  onClose: () => void;
}

// Standalone About dialog (Help → About ARMTEMP) — real Core Temp puts About
// under Help, not inside Settings.
export function AboutDialog({ onClose }: Props) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-small" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">About ARMTEMP</span>
          <div className="modal-spacer" />
          <button className="win-btn close-btn" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="tab-panel">
          <div className="about-head">
            <div className="about-logo">°</div>
            <div>
              <div className="about-name">ARMTEMP</div>
              <div className="about-version">Version 0.2.0 · ARM64 build · Tauri</div>
            </div>
          </div>
          <div className="groupbox dlg-group">
            <span className="groupbox-legend">Detected processor</span>
            <ChipsList />
          </div>
          <p className="about-note">
            Temperatures read from on-die thermal sensors via ACPI thermal zones. Per-core
            temps are real zone readings mapped to cores (not true per-core sensors on this
            firmware). ARMTEMP is an independent monitoring utility and is not affiliated
            with any silicon vendor.
          </p>
        </div>

        <div className="dlg-buttons">
          <button className="btn" onClick={onClose}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}
