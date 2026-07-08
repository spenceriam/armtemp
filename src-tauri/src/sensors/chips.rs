//! Chip profiles for the Snapdragon X / X2 families. This is the real silicon
//! reference table ported from the Claude design's `chips` array. It is used to
//! LABEL detected hardware (name, model, TjMax, cluster layout for P/E core
//! coloring) — never to fabricate readings. The actual CPU is detected at
//! runtime from the registry + `GetSystemInfo` (see `sensors/pdh.rs`) and
//! matched against these profiles.

use crate::sensors::types::CoreKind;

/// A known Snapdragon X-family SoC profile.
#[derive(Clone, Debug)]
pub struct ChipProfile {
    pub clusters: &'static [(CoreKind, u32)],
    pub id: &'static str,
    pub name: &'static str,
    /// Regex-style substring matched (lowercase, alphanumeric-only — see
    /// `normalize_cpu_name`) against the detected CPU name.
    pub model_match: &'static str,
    /// Marketing model string.
    pub model: &'static str,
    /// Thermal junction max in °C.
    pub tjmax_c: f64,
    /// Base clock GHz (informational; not always real).
    pub base_ghz: f64,
    /// Boost clock GHz (informational).
    pub boost_ghz: f64,
    /// Nominal TDP in watts (informational). 0 means unpublished/unknown —
    /// the UI renders that as an honest "—" (see `pdh.rs`'s `tdp_w` mapping).
    pub tdp_w: u32,
    /// Microarchitecture label, e.g. "Oryon" (X/X Plus/X Elite) or "Oryon 3"
    /// (X2 family, Qualcomm's 3rd Gen Oryon CPU). Empty string for the
    /// generic/unknown fallback profile.
    pub uarch: &'static str,
    /// Process node, e.g. "4 nm" (spec label describing the detected chip
    /// model — not a live telemetry reading). Empty for the generic fallback.
    pub lithography: &'static str,
}

impl ChipProfile {
    pub fn total_cores(&self) -> u32 {
        self.clusters.iter().map(|(_, n)| *n).sum()
    }
}

/// Keep only ASCII alphanumerics, lowercased. Makes chip-name matching immune
/// to hyphens, spaces, and "(R)"/"(TM)" marks that vary across OEM firmware
/// (e.g. `Snapdragon(R) X2 Elite - X2E-78-100` vs `X2E78100`).
pub fn normalize_cpu_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The real Snapdragon X / X2 lineup (matches the design and real silicon).
/// X2 specs are sourced from Qualcomm's official X2 Elite / X2 Plus product
/// briefs. Qualcomm does not publish a TDP for any X2 SKU (OEM-defined power
/// limits vary widely), so `tdp_w` is 0 (renders as "—") for the whole family;
/// TjMax is not published either and 105.0 is carried over from the X1 value
/// plus a small margin as a placeholder pending real-hardware confirmation.
pub static CHIPS: &[ChipProfile] = &[
    ChipProfile {
        id: "X",
        name: "Snapdragon X",
        model_match: "x126",
        model: "X1-26-100",
        tjmax_c: 100.0,
        base_ghz: 3.0,
        boost_ghz: 3.0,
        tdp_w: 23,
        clusters: &[(CoreKind::Performance, 8)],
        uarch: "Oryon",
        lithography: "4 nm",
    },
    ChipProfile {
        id: "XP",
        name: "Snapdragon X Plus",
        // X1P64100 (this machine), X1P-64-100 family.
        model_match: "x1p64",
        model: "X1P-64-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 3.4,
        tdp_w: 28,
        clusters: &[(CoreKind::Performance, 10)],
        uarch: "Oryon",
        lithography: "4 nm",
    },
    ChipProfile {
        id: "XP8",
        name: "Snapdragon X Plus",
        // 8-core X1P-80-100 (decoded differently to avoid colliding with Elite).
        model_match: "x1p80",
        model: "X1P-80-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 4.0,
        tdp_w: 30,
        clusters: &[(CoreKind::Performance, 8)],
        uarch: "Oryon",
        lithography: "4 nm",
    },
    ChipProfile {
        id: "XE",
        name: "Snapdragon X Elite",
        model_match: "x1e",
        model: "X1E-80-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 4.0,
        tdp_w: 37,
        clusters: &[(CoreKind::Performance, 12)],
        uarch: "Oryon",
        lithography: "4 nm",
    },
    // --- Snapdragon X2 family (3rd Gen Oryon, 3 nm) ---------------------
    // Cluster naming: Qualcomm's X2 marketing calls the two clusters "Prime"
    // and "Performance" (there is no "Efficiency" cluster name in X2 specs).
    // This app only has two `CoreKind`s, so Prime maps to `Performance`
    // (fastest cluster = "P" color) and the lower "Performance" cluster maps
    // to `Efficiency` — this preserves the existing P/E core-coloring
    // semantics rather than introducing a third tier.
    ChipProfile {
        id: "X2EX96",
        name: "Snapdragon X2 Elite Extreme",
        model_match: "x2e96",
        model: "X2E-96-100",
        tjmax_c: 105.0,
        base_ghz: 4.4,
        boost_ghz: 5.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2EX94",
        name: "Snapdragon X2 Elite Extreme",
        model_match: "x2e94",
        model: "X2E-94-100",
        tjmax_c: 105.0,
        base_ghz: 4.4,
        boost_ghz: 4.7,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2E90",
        name: "Snapdragon X2 Elite",
        model_match: "x2e90",
        model: "X2E-90-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 5.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2E88",
        name: "Snapdragon X2 Elite",
        model_match: "x2e88",
        model: "X2E-88-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.7,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2E84",
        name: "Snapdragon X2 Elite",
        model_match: "x2e84",
        model: "X2E-84-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.7,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2E80",
        name: "Snapdragon X2 Elite",
        model_match: "x2e80",
        model: "X2E-80-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.7,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2E78",
        name: "Snapdragon X2 Elite",
        model_match: "x2e78",
        model: "X2E-78-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2P64",
        name: "Snapdragon X2 Plus",
        model_match: "x2p64",
        model: "X2P-64-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 4)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
    ChipProfile {
        id: "X2P42",
        name: "Snapdragon X2 Plus",
        model_match: "x2p42",
        model: "X2P-42-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 4.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 6)],
        uarch: "Oryon 3",
        lithography: "3 nm",
    },
];

/// Build an even Prime/Performance cluster split for an X2 SKU that isn't in
/// `CHIPS` (a newer/unreleased part). `total` is the detected logical core
/// count. Mirrors the real lineup's split ratios: 18-core parts are 12+6,
/// 12-core parts are 6+6, 10-core parts are 6+4, everything else falls back
/// to an even split (or all-Performance for small counts).
fn x2_family_clusters(total: u32) -> &'static [(CoreKind, u32)] {
    let split = match total {
        18 => (12, 6),
        12 => (6, 6),
        10 => (6, 4),
        n if n >= 8 => (n * 2 / 3, n - n * 2 / 3),
        n => (n, 0),
    };
    if split.1 == 0 {
        Box::leak(vec![(CoreKind::Performance, split.0)].into_boxed_slice())
    } else {
        Box::leak(
            vec![
                (CoreKind::Performance, split.0),
                (CoreKind::Efficiency, split.1),
            ]
            .into_boxed_slice(),
        )
    }
}

/// Match a detected CPU name string against the profile table. `detected_name`
/// need not be pre-normalized — this normalizes internally. Returns the
/// matched profile; an unrecognized X2 SKU still gets a correctly-named X2
/// Elite/Plus family profile (not a generic fallback); anything else gets a
/// generic Snapdragon fallback with the detected core count split evenly.
/// Always returns something usable.
pub fn match_profile(detected_name: &str, core_count: u32) -> ChipProfile {
    let normalized = normalize_cpu_name(detected_name);
    for c in CHIPS {
        if normalized.contains(c.model_match) {
            return c.clone();
        }
    }
    // Unknown X2 SKU: still label the family correctly instead of "Generic".
    if normalized.contains("x2e") {
        return ChipProfile {
            id: "X2E",
            name: "Snapdragon X2 Elite",
            model_match: "",
            model: "Unknown X2 Elite SKU",
            tjmax_c: 105.0,
            base_ghz: 0.0,
            boost_ghz: 0.0,
            tdp_w: 0,
            clusters: x2_family_clusters(core_count),
            uarch: "Oryon 3",
            lithography: "3 nm",
        };
    }
    if normalized.contains("x2p") {
        return ChipProfile {
            id: "X2P",
            name: "Snapdragon X2 Plus",
            model_match: "",
            model: "Unknown X2 Plus SKU",
            tjmax_c: 105.0,
            base_ghz: 0.0,
            boost_ghz: 0.0,
            tdp_w: 0,
            clusters: x2_family_clusters(core_count),
            uarch: "Oryon 3",
            lithography: "3 nm",
        };
    }
    // Generic fallback: Snapdragon detected but unknown SKU/family. Even P/E
    // split when there are many cores; all-P otherwise.
    let (clusters, tjmax): (&'static [(CoreKind, u32)], f64) = if core_count >= 12 {
        (
            Box::leak(
                vec![
                    (CoreKind::Performance, core_count * 2 / 3),
                    (CoreKind::Efficiency, core_count - core_count * 2 / 3),
                ]
                .into_boxed_slice(),
            ),
            105.0,
        )
    } else {
        (
            Box::leak(vec![(CoreKind::Performance, core_count)].into_boxed_slice()),
            100.0,
        )
    };
    ChipProfile {
        id: "GEN",
        name: "Snapdragon",
        model_match: "",
        model: "Generic",
        tjmax_c: tjmax,
        base_ghz: 0.0,
        boost_ghz: 0.0,
        tdp_w: 0,
        clusters,
        uarch: "",
        lithography: "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x2_elite_78_real_world_format_matches() {
        // Real-world registry ProcessorNameString format (no hyphens).
        let p = match_profile(
            "Snapdragon(R) X2 Elite - X2E78100 - Qualcomm(R) Oryon(TM) CPU",
            12,
        );
        assert_eq!(p.model, "X2E-78-100");
        assert_eq!(p.name, "Snapdragon X2 Elite");
        assert_eq!(p.total_cores(), 12);
        assert_eq!(p.clusters, &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)]);
    }

    #[test]
    fn x2_elite_78_hyphenated_variant_matches() {
        let p = match_profile("Snapdragon(R) X2 Elite - X2E-78-100", 12);
        assert_eq!(p.model, "X2E-78-100");
    }

    #[test]
    fn x2_elite_88_18core_matches() {
        let p = match_profile("Snapdragon X2 Elite - X2E88100", 18);
        assert_eq!(p.model, "X2E-88-100");
        assert_eq!(p.total_cores(), 18);
        assert_eq!(p.clusters, &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)]);
    }

    #[test]
    fn x2_plus_64_10core_matches() {
        let p = match_profile("Snapdragon X2 Plus - X2P64100", 10);
        assert_eq!(p.model, "X2P-64-100");
        assert_eq!(p.total_cores(), 10);
    }

    #[test]
    fn unknown_x2_elite_sku_gets_family_profile_not_generic() {
        let p = match_profile("Snapdragon X2 Elite - X2E99100", 18);
        assert_eq!(p.name, "Snapdragon X2 Elite");
        assert_ne!(p.model, "Generic");
        assert_eq!(p.uarch, "Oryon 3");
        assert_eq!(p.tjmax_c, 105.0);
        assert_eq!(p.total_cores(), 18);
        assert_eq!(p.clusters, &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)]);
    }

    #[test]
    fn unknown_x2_plus_sku_gets_family_profile_not_generic() {
        let p = match_profile("Snapdragon X2 Plus - X2P99100", 6);
        assert_eq!(p.name, "Snapdragon X2 Plus");
        assert_ne!(p.model, "Generic");
        assert_eq!(p.uarch, "Oryon 3");
    }

    #[test]
    fn x1_plus_64_regression() {
        let p = match_profile("Snapdragon(R) X Plus - X1P64100", 10);
        assert_eq!(p.model, "X1P-64-100");
        assert_eq!(p.name, "Snapdragon X Plus");
    }

    #[test]
    fn x1_26_regression_guards_key_change() {
        // Guards the "x1-26" -> "x126" model_match key change.
        let p = match_profile("Snapdragon X - X1-26-100", 8);
        assert_eq!(p.model, "X1-26-100");
        assert_eq!(p.name, "Snapdragon X");
    }

    #[test]
    fn x1_elite_regression() {
        let p = match_profile("Snapdragon(R) X Elite - X1E80100", 12);
        assert_eq!(p.model, "X1E-80-100");
    }

    #[test]
    fn non_snapdragon_falls_back_to_generic_even_split() {
        let p = match_profile("Some Other CPU", 8);
        assert_eq!(p.model, "Generic");
        assert_eq!(p.name, "Snapdragon");
        assert_eq!(p.total_cores(), 8);
    }
}
