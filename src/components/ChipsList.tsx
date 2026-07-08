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
    { name: "Snapdragon X Plus", model: "X1P-80-100", cores: "8 cores" },
    { name: "Snapdragon X Elite", model: "X1E-80-100", cores: "12 cores" },
    { name: "Snapdragon X2 Elite Extreme", model: "X2E-96-100", cores: "18 cores" },
    { name: "Snapdragon X2 Elite Extreme", model: "X2E-94-100", cores: "18 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-90-100", cores: "18 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-88-100", cores: "18 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-84-100", cores: "12 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-80-100", cores: "12 cores" },
    { name: "Snapdragon X2 Elite", model: "X2E-78-100", cores: "12 cores" },
    { name: "Snapdragon X2 Plus", model: "X2P-64-100", cores: "10 cores" },
    { name: "Snapdragon X2 Plus", model: "X2P-42-100", cores: "6 cores" },
  ];

  // Highlighted by `model`, not `name` — several X2 SKUs share the same
  // marketing name, so model (the unique part number) is the real key.
  const detectedMatch = known.find(
    (k) => profile && k.model === profile.model
  );

  return (
    <div className="chips-list">
      {known.map((c) => {
        const isDetected = detectedMatch && detectedMatch.model === c.model;
        return (
          <label
            key={c.model}
            className={`chk-row chk-static ${isDetected ? "" : "chk-dim"}`}
            title={isDetected ? "Detected on this machine" : "Not detected"}
          >
            <input type="radio" checked={!!isDetected} readOnly />
            <span>
              {c.name}
              {isDetected ? " · detected" : ""}{" "}
              <span className="chk-sub">
                ({c.model} · {c.cores})
              </span>
            </span>
          </label>
        );
      })}
      {profile && !detectedMatch && (
        <label className="chk-row chk-static">
          <input type="radio" checked readOnly />
          <span>
            {profile.name} · detected{" "}
            <span className="chk-sub">
              ({profile.model} · {profile.cores} cores)
            </span>
          </span>
        </label>
      )}
    </div>
  );
}
