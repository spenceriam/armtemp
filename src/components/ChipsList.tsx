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

// Shows only the REAL detected processor (from the Rust backend) — not the
// full known Snapdragon X/X2 lineup, which would grow unbounded as new
// families (X3, …) ship and reads as noise once you already know what's
// detected.
export function ChipsList() {
  const [profile, setProfile] = useState<Profile | null>(null);
  useEffect(() => {
    invoke<Profile>("get_profile")
      .then(setProfile)
      .catch(() => setProfile(null));
  }, []);

  if (!profile) {
    return (
      <div className="chips-list">
        <div className="chk-row chk-static chk-dim">
          <span>Detecting…</span>
        </div>
      </div>
    );
  }

  // "Unknown … SKU" / "Generic" aren't real model numbers — same rule as
  // ProcessorInfo.tsx's `isPlaceholderModel`, so an unconfirmed SKU doesn't
  // read as "Snapdragon X2 Elite (Unknown X2 Elite SKU · 12 cores)".
  const isPlaceholderModel = !profile.model || /^unknown\b/i.test(profile.model) || profile.model === "Generic";

  return (
    <div className="chips-list">
      <div className="chk-row chk-static">
        {profile.name}{" "}
        <span className="chk-sub">
          ({isPlaceholderModel ? `${profile.cores} cores` : `${profile.model} · ${profile.cores} cores`})
        </span>
      </div>
    </div>
  );
}
