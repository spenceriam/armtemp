//! Chip profiles for the Snapdragon X / X2 families. This is the real silicon
//! reference table ported from the Clod design's `chips` array. It is used to
//! LABEL detected hardware (name, model, TjMax, cluster layout for P/E core
//! coloring) — never to fabricate readings. The actual CPU is detected at
//! runtime from Win32_Processor and matched against these profiles.

use crate::sensors::types::CoreKind;

/// A known Snapdragon X-family SoC profile.
#[derive(Clone, Debug)]
pub struct ChipProfile {
    pub clusters: &'static [(CoreKind, u32)],
    pub id: &'static str,
    pub name: &'static str,
    /// Regex-style substring matched (lowercase) against the detected CPU name.
    pub model_match: &'static str,
    /// Marketing model string.
    pub model: &'static str,
    /// Thermal junction max in °C.
    pub tjmax_c: f64,
    /// Base clock GHz (informational; not always real).
    pub base_ghz: f64,
    /// Boost clock GHz (informational).
    pub boost_ghz: f64,
    /// Nominal TDP in watts (informational).
    pub tdp_w: u32,
}

impl ChipProfile {
    pub fn total_cores(&self) -> u32 {
        self.clusters.iter().map(|(_, n)| *n).sum()
    }
}

/// The real Snapdragon X / X2 lineup (matches the design and real silicon).
pub static CHIPS: &[ChipProfile] = &[
    ChipProfile {
        id: "X",
        name: "Snapdragon X",
        model_match: "x1-26",
        model: "X1-26-100",
        tjmax_c: 100.0,
        base_ghz: 3.0,
        boost_ghz: 3.0,
        tdp_w: 23,
        clusters: &[(CoreKind::Performance, 8)],
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
    },
    ChipProfile {
        id: "X2",
        name: "Snapdragon X2",
        model_match: "x2-46",
        model: "X2-46-100",
        tjmax_c: 105.0,
        base_ghz: 3.4,
        boost_ghz: 3.6,
        tdp_w: 25,
        clusters: &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)],
    },
    ChipProfile {
        id: "X2P",
        name: "Snapdragon X2 Plus",
        model_match: "x2p66",
        model: "X2P-66-100",
        tjmax_c: 105.0,
        base_ghz: 3.8,
        boost_ghz: 4.2,
        tdp_w: 40,
        clusters: &[(CoreKind::Performance, 8), (CoreKind::Efficiency, 8)],
    },
    ChipProfile {
        id: "X2E",
        name: "Snapdragon X2 Elite",
        model_match: "x2e88",
        model: "X2E-88-100",
        tjmax_c: 105.0,
        base_ghz: 4.0,
        boost_ghz: 5.0,
        tdp_w: 50,
        clusters: &[(CoreKind::Performance, 12), (CoreKind::Efficiency, 6)],
    },
];

/// Match a detected CPU name string (lowercased) against the profile table.
/// Returns the matched profile, or a generic Snapdragon fallback with the
/// detected core count split evenly. Always returns something usable.
pub fn match_profile(detected_name_lower: &str, core_count: u32) -> ChipProfile {
    for c in CHIPS {
        if detected_name_lower.contains(c.model_match) {
            return c.clone();
        }
    }
    // Generic fallback: Snapdragon detected but unknown SKU. Even P/E split
    // when there are many cores; all-P otherwise.
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
    }
}
