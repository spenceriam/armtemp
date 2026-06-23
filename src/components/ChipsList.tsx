import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Profile {
  name: string;
  model: string;
  tjmax_c: number;
  base_ghz: number;
  boost_ghz: number;
  tdp_w: number;
  cores: number;
}

// Shows the REAL detected processor (from the Rust backend) plus the known
// Snapdragon X / X2 family as reference. The detected one is highlighted.
export function ChipsList() {
  const [profile, setProfile] = useState<Profile | null>(null);
  useEffect(() => {
    invoke<Profile>("get_profile")
      .then(setProfile)
      .catch(() => setProfile(null));
  }, []);

  const known: { name: string; model: string; cores: string }[] = [
    { name: "Snapdragon X", model: "X1-26-100", cores: "8 cores" },
    { name: "Snapdragon X Plus", model: "X1P-64-100", cores: "10 cores" },
    { name: "Snapdragon X Elite", model: "X1E-80-100", cores: "12 cores" },
    { name: "Snapdragon X2", model: "X2-46-100", cores: "12 cores" },
    { name: "Snapdragon X2 Plus", model: "X2P-66-100", cores: "16 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-88-100", cores: "18 cores" },
  ];

  const detectedMatch = known.find(
    (k) => profile && k.name === profile.name
  );

  return (
    <div className="chips-list">
      {known.map((c) => {
        const isDetected = detectedMatch && detectedMatch.name === c.name;
        return (
          <button
            key={c.model}
            className={`radio ${isDetected ? "active" : ""}`}
            disabled
            title={isDetected ? "Detected on this machine" : "Not detected"}
          >
            <div className={`radio-dot ${isDetected ? "active" : ""}`} />
            <div>
              <div className="radio-label">
                {c.name}
                {isDetected ? " · detected" : ""}
              </div>
              <div className="radio-sub">
                {c.model} · {c.cores}
              </div>
            </div>
          </button>
        );
      })}
      {profile && !detectedMatch && (
        <div className="radio active">
          <div className="radio-dot active" />
          <div>
            <div className="radio-label">{profile.name} · detected</div>
            <div className="radio-sub">
              {profile.model} · {profile.cores} cores
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
