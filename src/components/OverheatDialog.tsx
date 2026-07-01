import { useRef } from "react";
import { AppSettings, OverheatAction } from "../app/types";

interface Props {
  settings: AppSettings;
  update: (patch: Partial<AppSettings>) => void;
  tjmax: number;
  onClose: () => void;
}

// Standalone "Overheat protection" dialog, opened from Options — real Core
// Temp configures this in its own dialog, not a Settings tab. Same live-apply
// + Cancel-reverts semantics as the Settings dialog.
export function OverheatDialog({ settings, update, tjmax, onClose }: Props) {
  const baseline = useRef({
    overheatOn: settings.overheatOn,
    overheatThreshold: settings.overheatThreshold,
    overheatAction: settings.overheatAction,
  });

  const maxC = Math.min(105, Math.round(tjmax));

  const cancel = () => {
    update(baseline.current);
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={cancel}>
      <div className="modal modal-small" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">Overheat protection</span>
          <div className="modal-spacer" />
          <button className="win-btn close-btn" onClick={cancel}>
            ✕
          </button>
        </div>

        <div className="tab-panel">
          <div className="groupbox dlg-group">
            <span className="groupbox-legend">Overheat protection</span>
            <label className="chk-row">
              <input
                type="checkbox"
                checked={settings.overheatOn}
                onChange={(e) => update({ overheatOn: e.target.checked })}
              />
              <span>Enable overheat protection</span>
            </label>
            <label className="select-row">
              <span>Activation temperature</span>
              <span className="num-wrap">
                <input
                  type="number"
                  className="num-input"
                  min={70}
                  max={maxC}
                  step={1}
                  disabled={!settings.overheatOn}
                  value={settings.overheatThreshold}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    if (Number.isFinite(n)) update({ overheatThreshold: n });
                  }}
                />
                <span className="num-unit">°C</span>
              </span>
            </label>
            <div className="dlg-hint">Tj. Max is {tjmax}°C — recommended ~5°C below.</div>
          </div>

          <div className="groupbox dlg-group">
            <span className="groupbox-legend">When temperature is reached</span>
            {(
              [
                ["notify", "Show a notification"],
                ["sleep", "Put the computer to sleep"],
                ["shutdown", "Shut the computer down"],
              ] as [OverheatAction, string][]
            ).map(([v, label]) => (
              <label className="chk-row" key={v}>
                <input
                  type="radio"
                  name="overheatAction"
                  disabled={!settings.overheatOn}
                  checked={settings.overheatAction === v}
                  onChange={() => update({ overheatAction: v })}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </div>

        <div className="dlg-buttons">
          <button className="btn" onClick={onClose}>
            OK
          </button>
          <button className="btn" onClick={cancel}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
