import { openUrl } from "@tauri-apps/plugin-opener";
import { AppIcon } from "./AppIcon";

interface Props {
  onClose: () => void;
}

const REPO_URL = "https://github.com/spenceriam/armtemp";
const X_URL = "https://x.com/spencer_i_am";
const SITE_URL = "https://spencer.build";

function open(url: string) {
  openUrl(url).catch(() => {});
}

// Standalone About dialog (Help → About ARMtemp) — credits card: logo, name,
// version, author + links. Real Core Temp puts About under Help, not inside
// Settings.
export function AboutDialog({ onClose }: Props) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-small" onClick={(e) => e.stopPropagation()}>
        <div className="modal-titlebar">
          <span className="modal-title">About ARMtemp</span>
          <div className="modal-spacer" />
          <button className="win-btn close-btn" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="tab-panel">
          <div className="about-head">
            <div className="about-logo">
              <AppIcon size={30} />
            </div>
            <div>
              <div className="about-name">ARMtemp</div>
              <div className="about-version">Version 0.3.1 · ARM64 build · Tauri</div>
            </div>
          </div>

          <div className="about-author">
            Built by Spencer Francisco
            <button
              className="about-x-link"
              onClick={() => open(X_URL)}
              title="@spencer_i_am on X"
              aria-label="@spencer_i_am on X"
            >
              <XIcon size={13} />
            </button>
          </div>
          <div className="about-links">
            <button className="about-link" onClick={() => open(SITE_URL)}>
              spencer.build
            </button>
            <span className="about-link-sep">·</span>
            <button className="about-link" onClick={() => open(REPO_URL)}>
              GitHub repo
            </button>
          </div>
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

function XIcon({ size = 14 }: { size?: number }) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor">
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}
