//! Chip profiles for ARM64 CPUs seen on Windows on ARM: the Snapdragon X / X2
//! families primarily, plus the legacy Qualcomm Kryo-based Windows-on-ARM
//! lineup (Snapdragon 835/850/7c family/8c family/8cx family, Microsoft
//! SQ1-3) and Raspberry Pi (Windows-on-ARM community installs), with honest
//! vendor-derived fallbacks for Broadcom/MediaTek/NVIDIA and anything else
//! unrecognized. This is used to LABEL detected hardware (name, model,
//! TjMax, cluster layout) — never to fabricate readings.
//!
//! Detection is layered, cheapest/most-certain evidence first (see
//! `match_profile`):
//!   A. SKU token in `ProcessorNameString` (e.g. "X2E78100", "8cx Gen 3",
//!      "SQ1") — exact, when present.
//!   B. Snapdragon X family/subfamily parsed from the name ("X2 Elite", "X2
//!      Elite Extreme", "X2 Plus", …), or — if the name carries no usable
//!      text at all — the registry `Identifier`'s MIDR part number (Oryon
//!      generation) plus vendor.
//!   C. Within that X-family, the exact SKU inferred from real, hardware-
//!      reported signals: logical core count, P/E topology, and each core's
//!      `~MHz` (rated/boost clock) — the same signals CPU-Z and HWiNFO use,
//!      since issue #2 showed some OEM firmware (Surface) reports a bare
//!      marketing name with no SKU token in it at all.
//!   D. X-family recognized but no SKU matches (unreleased/future part):
//!      label the family honestly, with boost populated from the real
//!      measured clock — never "Generic".
//!   E. A known non-X-series vendor/family (legacy Qualcomm Kryo chips,
//!      Broadcom, MediaTek, NVIDIA) but no specific SKU token matched:
//!      honest vendor/family label, never invented specs.
//!   F. Nothing recognized at all: honest vendor-derived (or bare "ARM64
//!      CPU") fallback — never "Snapdragon Generic" for a chip that isn't one.
//!
//! The actual CPU identity is gathered in `sensors::identity` (registry +
//! `GetSystemInfo`/topology query in `sensors::pdh`).
//!
//! Core-tier vocabulary (see `CoreNaming`/`ChipProfile::tier_badge`): the
//! label shown for the two `CoreKind` tiers depends on the real silicon, not
//! a one-size-fits-all "Performance/Efficiency". Snapdragon X1 has no
//! efficiency cores at all (uniform Oryon cores; clusters differ only in
//! clock cap) — every core badges "P". Snapdragon X2's own vocabulary is
//! "Prime"/"Performance", neither an efficiency core — badged "P"/"P2".
//! Legacy Kryo-based Qualcomm chips, and anything else with a genuine
//! big.LITTLE split, keep the accurate "P"/"E".

use crate::sensors::identity::CpuIdentity;
use crate::sensors::types::CoreKind;

/// Vocabulary for the two core tiers a chip may expose. See the module doc
/// for the research behind each variant — this exists so the UI never
/// misrepresents real silicon with borrowed Intel/generic wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreNaming {
    /// Every core is a full performance core (Snapdragon X1 family: uniform
    /// Oryon cores, no efficiency tier — clusters differ only in clock cap).
    UniformPerformance,
    /// Qualcomm's own X2-family vocabulary: faster "Prime" cluster, slower
    /// "Performance" cluster — neither is an efficiency core.
    PrimePerformance,
    /// A genuine Performance/Efficiency split: legacy Kryo-based Qualcomm
    /// chips, other hybrid ARM designs, and anything unrecognized (where we
    /// can't assert the chip has no efficiency tier).
    PerformanceEfficiency,
}

/// A known ARM64 SoC profile.
#[derive(Clone, Debug)]
pub struct ChipProfile {
    pub clusters: &'static [(CoreKind, u32)],
    pub id: &'static str,
    pub name: &'static str,
    /// Alphanumeric-lowercase substring matched against the normalized
    /// detected CPU name (see `CpuIdentity::normalized_name`). Empty for
    /// profiles that are never reached via name-token matching (Stage B/D/E/F
    /// results).
    pub model_match: &'static str,
    /// Marketing model string.
    pub model: &'static str,
    /// Thermal junction max in °C.
    pub tjmax_c: f64,
    /// Base (all-core) clock GHz (informational; not always real). 0 means
    /// unpublished/unverified — renders as "—" (see `pdh.rs`).
    pub base_ghz: f64,
    /// Boost clock GHz (informational).
    pub boost_ghz: f64,
    /// Nominal TDP in watts (informational). 0 means unpublished/unverified —
    /// the UI renders that as an honest "—" (see `pdh.rs`'s `tdp_w` mapping).
    pub tdp_w: u32,
    /// Microarchitecture label, e.g. "Oryon" (X1 family) or "Oryon 3" (X2
    /// family, Qualcomm's 3rd Gen Oryon). Empty string when unknown.
    pub uarch: &'static str,
    /// Process node, e.g. "4 nm" (spec label describing the detected chip
    /// model — not a live telemetry reading). Empty when unpublished/unknown.
    pub lithography: &'static str,
    /// Which core-tier vocabulary applies to this chip — see `CoreNaming`.
    pub core_naming: CoreNaming,
}

impl ChipProfile {
    pub fn total_cores(&self) -> u32 {
        self.clusters.iter().map(|(_, n)| *n).sum()
    }

    /// The badge text + tooltip to show for a core of the given `CoreKind`
    /// on this chip, per its `core_naming`. This is the single place that
    /// decides "P"/"P2"/"E" vocabulary — never hardcode it elsewhere.
    pub fn tier_badge(&self, kind: CoreKind) -> (&'static str, &'static str) {
        match (self.core_naming, kind) {
            (CoreNaming::UniformPerformance, _) => ("P", "Performance core"),
            (CoreNaming::PrimePerformance, CoreKind::Performance) => ("P", "Prime core"),
            (CoreNaming::PrimePerformance, CoreKind::Efficiency) => ("P2", "Performance core"),
            (CoreNaming::PerformanceEfficiency, CoreKind::Performance) => ("P", "Performance core"),
            (CoreNaming::PerformanceEfficiency, CoreKind::Efficiency) => ("E", "Efficiency core"),
        }
    }
}

/// How a `ChipProfile` was determined — surfaced to the UI/diagnostics report
/// so "we're not sure" is never presented as "we're sure".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchBasis {
    /// An exact SKU token (e.g. "x2e78", "8cxgen3", "sq1") was found in the
    /// CPU name.
    SkuToken,
    /// No SKU token in the name; the exact SKU was inferred from real core
    /// count + real rated clock within an X-family identified from the name
    /// (or Identifier).
    Inferred,
    /// The family/subfamily (or, for non-X-series vendors, the vendor) is
    /// known but no specific SKU matched (unreleased part, an ambiguous
    /// pair, or a recognized vendor with no token hit).
    FamilyOnly,
    /// Nothing recognized at all.
    Generic,
}

impl MatchBasis {
    /// Short human-readable label for the UI/diagnostics report.
    pub fn label(&self) -> &'static str {
        match self {
            MatchBasis::SkuToken => "exact SKU token in CPU name",
            MatchBasis::Inferred => "inferred from core count + rated clock",
            MatchBasis::FamilyOnly => "family recognized, SKU unconfirmed",
            MatchBasis::Generic => "unrecognized",
        }
    }
}

pub struct Detection {
    pub profile: ChipProfile,
    pub basis: MatchBasis,
}

/// The real Snapdragon X / X2 lineup, the legacy Qualcomm Windows-on-ARM
/// lineup, and Raspberry Pi (Windows-on-ARM community installs).
///
/// Snapdragon X specs are sourced from Qualcomm's X/X2 Elite/Plus product
/// briefs and notebookcheck's per-SKU pages. Legacy Qualcomm specs (835, 850,
/// 7c family, 8c family, 8cx family, Microsoft SQ1-3) are sourced from
/// Qualcomm/Microsoft product pages, Wikipedia's Snapdragon SoC list, and
/// PassMark/HWiNFO-verified `ProcessorNameString` formats. Raspberry Pi specs
/// are from Raspberry Pi's own documentation. TDP is unpublished for the
/// entire legacy/Pi lineup — `tdp_w` is 0 (renders "—") rather than a guess;
/// `base_ghz` likewise where no distinct all-core clock was found.
pub static CHIPS: &[ChipProfile] = &[
    // --- Snapdragon X family (1st Gen Oryon, 4 nm) ----------------------
    ChipProfile {
        id: "X126",
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
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1P42",
        name: "Snapdragon X Plus",
        model_match: "x1p42",
        model: "X1P-42-100",
        tjmax_c: 100.0,
        base_ghz: 3.2,
        boost_ghz: 3.4,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 8)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1P46",
        name: "Snapdragon X Plus",
        model_match: "x1p46",
        model: "X1P-46-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 4.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 8)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1P64",
        name: "Snapdragon X Plus",
        // X1P64100 (this dev machine).
        model_match: "x1p64",
        model: "X1P-64-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 3.4,
        tdp_w: 28,
        clusters: &[(CoreKind::Performance, 10)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1P66",
        name: "Snapdragon X Plus",
        model_match: "x1p66",
        model: "X1P-66-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 4.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 10)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1E78",
        name: "Snapdragon X Elite",
        model_match: "x1e78",
        model: "X1E-78-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 3.4, // no dual-core boost above base — the entry-tier Elite.
        tdp_w: 35,
        clusters: &[(CoreKind::Performance, 12)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1E80",
        name: "Snapdragon X Elite",
        model_match: "x1e80",
        model: "X1E-80-100",
        tjmax_c: 100.0,
        base_ghz: 3.4,
        boost_ghz: 4.0,
        tdp_w: 37,
        clusters: &[(CoreKind::Performance, 12)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "X1E84",
        name: "Snapdragon X Elite",
        model_match: "x1e84",
        model: "X1E-84-100",
        tjmax_c: 100.0,
        base_ghz: 3.8,
        boost_ghz: 4.2,
        tdp_w: 45,
        clusters: &[(CoreKind::Performance, 12)],
        uarch: "Oryon",
        lithography: "4 nm",
        core_naming: CoreNaming::UniformPerformance,
    },
    // --- Snapdragon X2 family (3rd Gen Oryon, 3 nm) ---------------------
    // Cluster naming: Qualcomm's X2 marketing calls the two clusters "Prime"
    // and "Performance" (there is no "Efficiency" cluster name in X2 specs).
    // Internally Prime maps to `CoreKind::Performance` and the lower
    // "Performance" cluster maps to `CoreKind::Efficiency` (preserves the
    // existing two-tier plumbing); `core_naming: PrimePerformance` is what
    // turns that back into the correct "P"/"P2" display vocabulary.
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
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
        core_naming: CoreNaming::PrimePerformance,
    },
    // --- Legacy Qualcomm Windows-on-ARM lineup (genuine big.LITTLE — "P"/"E"
    // is accurate vocabulary here, unlike the Oryon families above). Ordered
    // most-specific-token-first: "8cxgen3"/"8cxgen2" before bare "8cx" before
    // "8c" (each a substring of the ones above it after normalization), and
    // "7cgen3"/"7cgen2" before bare "7c" — `match_profile`'s Stage A returns
    // the FIRST array match, so order is load-bearing here.
    ChipProfile {
        id: "SQ3",
        name: "Microsoft SQ3",
        model_match: "microsoftsq3",
        model: "SQ3",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 3.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Cortex-X1C + Cortex-A78C",
        lithography: "5 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SQ2",
        name: "Microsoft SQ2",
        model_match: "microsoftsq2",
        model: "SQ2",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 3.15,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 495",
        lithography: "7 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SQ1",
        name: "Microsoft SQ1",
        model_match: "microsoftsq1",
        model: "SQ1",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 3.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 495",
        lithography: "7 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC8180XP",
        name: "Snapdragon 8cx Gen 3",
        model_match: "8cxgen3",
        model: "SC8280XP",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 3.0,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Cortex-X1C + Cortex-A78C",
        lithography: "5 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC8180XG2",
        name: "Snapdragon 8cx Gen 2",
        model_match: "8cxgen2",
        model: "SC8180X+",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 3.15,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 495",
        lithography: "7 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC8180X",
        name: "Snapdragon 8cx",
        model_match: "8cx",
        model: "SC8180X",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.84,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 495",
        lithography: "7 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC8180",
        name: "Snapdragon 8c",
        model_match: "8c",
        model: "SC8180",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.45,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 490",
        lithography: "7 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC7280",
        name: "Snapdragon 7c+ Gen 3",
        model_match: "7cgen3",
        model: "SC7280",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.4,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo (7c+ Gen 3)",
        lithography: "6 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC7180P",
        name: "Snapdragon 7c Gen 2",
        model_match: "7cgen2",
        model: "SC7180P",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.55,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 2), (CoreKind::Efficiency, 6)],
        uarch: "Kryo 468",
        lithography: "8 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SC7180",
        name: "Snapdragon 7c",
        model_match: "7c",
        model: "SC7180",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.4,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 2), (CoreKind::Efficiency, 6)],
        uarch: "Kryo 468",
        lithography: "8 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "SDM850",
        name: "Snapdragon 850",
        model_match: "snapdragon850",
        model: "SDM850",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.96,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 385",
        lithography: "10 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    ChipProfile {
        id: "MSM8998",
        name: "Snapdragon 835",
        model_match: "snapdragon835",
        model: "MSM8998",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.45,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4), (CoreKind::Efficiency, 4)],
        uarch: "Kryo 280",
        lithography: "10 nm",
        core_naming: CoreNaming::PerformanceEfficiency,
    },
    // --- Raspberry Pi (Windows-on-ARM community installs) — homogeneous. ---
    ChipProfile {
        id: "BCM2711",
        name: "Broadcom BCM2711",
        model_match: "bcm2711",
        model: "BCM2711",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 1.5,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4)],
        uarch: "Cortex-A72",
        lithography: "",
        core_naming: CoreNaming::UniformPerformance,
    },
    ChipProfile {
        id: "BCM2712",
        name: "Broadcom BCM2712",
        model_match: "bcm2712",
        model: "BCM2712",
        tjmax_c: 100.0,
        base_ghz: 0.0,
        boost_ghz: 2.4,
        tdp_w: 0,
        clusters: &[(CoreKind::Performance, 4)],
        uarch: "Cortex-A76",
        lithography: "",
        core_naming: CoreNaming::UniformPerformance,
    },
];

fn chip_by_id(id: &str) -> ChipProfile {
    CHIPS
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .unwrap_or_else(|| panic!("chips.rs: no CHIPS entry with id {id}"))
}

/// X2E-80-100 and X2E-84-100 publish IDENTICAL CPU-visible specs (same 4.7
/// GHz boost, same 34 MB cache, same core split) — notebookcheck's only
/// stated difference is dual-core boost (4.4 vs 4.7 GHz) and NPU TOPS rating,
/// neither of which is readable from this app's data sources. Rather than
/// guess, both map to one honestly-labeled combined profile.
fn x2e_80_84_ambiguous() -> ChipProfile {
    let base = chip_by_id("X2E80");
    ChipProfile {
        id: "X2E80OR84",
        model_match: "",
        model: "X2E-80/84-100",
        ..base
    }
}

/// Build an even higher/lower-tier cluster split for a chip that isn't in
/// `CHIPS` (an unreleased/unlisted part, or an unconfirmed vendor family).
/// `total` is the detected logical core count. Mirrors the X2 lineup's real
/// split ratios: 18-core parts are 12+6, 12-core parts are 6+6, 10-core
/// parts are 6+4, everything else falls back to an even split (or all-P for
/// small counts).
fn heuristic_pe_clusters(total: u32) -> &'static [(CoreKind, u32)] {
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

fn homogeneous_cluster(total: u32) -> &'static [(CoreKind, u32)] {
    Box::leak(vec![(CoreKind::Performance, total)].into_boxed_slice())
}

/// Prefer the OS's real per-core P/E topology over any static table split.
/// Falls back to `fallback` when the topology wasn't available or its counts
/// don't add up to the detected logical core count.
fn resolve_clusters(
    id: &CpuIdentity,
    cores: u32,
    fallback: &'static [(CoreKind, u32)],
) -> &'static [(CoreKind, u32)] {
    if let (Some(p), Some(e)) = (id.perf_cores, id.eff_cores) {
        if cores > 0 && p + e == cores {
            let mut v = Vec::new();
            if p > 0 {
                v.push((CoreKind::Performance, p));
            }
            if e > 0 {
                v.push((CoreKind::Efficiency, e));
            }
            if !v.is_empty() {
                return Box::leak(v.into_boxed_slice());
            }
        }
    }
    fallback
}

/// Pick the candidate whose published clock is closest to `measured_mhz`,
/// rejecting the match entirely if even the closest one is further than
/// `tolerance_mhz` away (an unrecognized SKU, not a noisy reading of a known
/// one — every known family+core-count group's neighboring SKUs are at least
/// twice `tolerance_mhz` apart).
fn pick_by_clock(
    measured_mhz: u32,
    candidates: Vec<(u32, ChipProfile)>,
    tolerance_mhz: u32,
) -> Option<ChipProfile> {
    candidates
        .into_iter()
        .min_by_key(|(mhz, _)| (*mhz as i64 - measured_mhz as i64).unsigned_abs())
        .filter(|(mhz, _)| (*mhz as i64 - measured_mhz as i64).unsigned_abs() as u32 <= tolerance_mhz)
        .map(|(_, p)| p)
}

const CLOCK_TOLERANCE_MHZ: u32 = 180;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gen {
    X1,
    X2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sub {
    Elite,
    Plus,
    Extreme,
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Guards the bare-"snapdragon" branch of `detect_family` below: every
/// legacy Qualcomm chip's marketing name also contains the word
/// "Snapdragon", so without this guard an unanticipated real-world string
/// format for one of THEM (e.g. "Snapdragon 850 @ 2.96 GHz", only 40 MHz off
/// the Snapdragon X 8-core's 3.0 GHz) could be misread as a Snapdragon X
/// part purely because its clock happens to land near 3.0 GHz. Each of these
/// chips already has its own Stage A token in `CHIPS`; this only matters as
/// a second line of defense.
const LEGACY_GUARD_TOKENS: &[&str] = &["835", "850", "7c", "8c", "sq1", "sq2", "sq3"];

/// Stage B: Snapdragon X family + subfamily from the CPU name; if the name
/// carries no usable text at all, fall back to the registry `Identifier`'s
/// MIDR part number (Oryon generation) — gated on the vendor string actually
/// saying Qualcomm, so a non-Snapdragon ARM chip that happens to share a
/// Model number never gets mislabeled. Returns `None` for anything that
/// isn't Snapdragon-X-shaped, including legacy Qualcomm chips (Stage E below
/// handles those by vendor).
fn detect_family(id: &CpuIdentity) -> Option<(Gen, Option<Sub>)> {
    let normalized = id.normalized_name();
    if !normalized.is_empty() {
        if normalized.contains("x2") {
            let sub = if normalized.contains("extreme") {
                Some(Sub::Extreme)
            } else if normalized.contains("plus") {
                Some(Sub::Plus)
            } else if normalized.contains("elite") {
                Some(Sub::Elite)
            } else {
                None
            };
            return Some((Gen::X2, sub));
        }
        if normalized.contains("x1") {
            let sub = if normalized.contains("elite") {
                Some(Sub::Elite)
            } else if normalized.contains("plus") {
                Some(Sub::Plus)
            } else {
                None
            };
            return Some((Gen::X1, sub));
        }
        if normalized.contains("snapdragon") && !contains_any(&normalized, LEGACY_GUARD_TOKENS) {
            return Some((Gen::X1, None));
        }
    }
    let vendor_is_qualcomm = id
        .vendor
        .as_deref()
        .map(|v| v.to_lowercase().contains("qualcomm"))
        .unwrap_or(false);
    if !vendor_is_qualcomm {
        return None;
    }
    match id.oryon_generation() {
        Some(2) => Some((Gen::X2, None)),
        Some(1) => Some((Gen::X1, None)),
        _ => None,
    }
}

fn x2_elite_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        18 => vec![(4700, chip_by_id("X2E88")), (5000, chip_by_id("X2E90"))],
        12 => vec![(4000, chip_by_id("X2E78")), (4700, x2e_80_84_ambiguous())],
        _ => vec![],
    }
}

fn x2_elite_extreme_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        18 => vec![(4700, chip_by_id("X2EX94")), (5000, chip_by_id("X2EX96"))],
        _ => vec![],
    }
}

fn x2_plus_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        10 => vec![(4000, chip_by_id("X2P64"))],
        6 => vec![(4000, chip_by_id("X2P42"))],
        _ => vec![],
    }
}

fn x1_elite_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        12 => vec![
            (3400, chip_by_id("X1E78")),
            (4000, chip_by_id("X1E80")),
            (4200, chip_by_id("X1E84")),
        ],
        _ => vec![],
    }
}

fn x1_plus_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        8 => vec![(3400, chip_by_id("X1P42")), (4000, chip_by_id("X1P46"))],
        10 => vec![(3400, chip_by_id("X1P64")), (4000, chip_by_id("X1P66"))],
        _ => vec![],
    }
}

fn x1_candidates(cores: u32) -> Vec<(u32, ChipProfile)> {
    match cores {
        8 => vec![(3000, chip_by_id("X126"))],
        _ => vec![],
    }
}

/// Stage C: within a known X-family/subfamily, infer the exact SKU from real
/// core count + real rated clock. `None` if the family has no candidates for
/// this core count, or the measured clock is too far from every candidate.
fn infer_within_family(gen: Gen, sub: Option<Sub>, id: &CpuIdentity) -> Option<(ChipProfile, MatchBasis)> {
    let cores = id.logical_cores;
    let candidates = match (gen, sub) {
        (Gen::X2, Some(Sub::Extreme)) => x2_elite_extreme_candidates(cores),
        (Gen::X2, Some(Sub::Plus)) => x2_plus_candidates(cores),
        (Gen::X2, Some(Sub::Elite)) | (Gen::X2, None) => x2_elite_candidates(cores),
        (Gen::X1, Some(Sub::Elite)) => x1_elite_candidates(cores),
        (Gen::X1, Some(Sub::Plus)) => x1_plus_candidates(cores),
        (Gen::X1, Some(Sub::Extreme)) => vec![], // no X1 "Extreme" tier exists
        (Gen::X1, None) => x1_candidates(cores),
    };
    // X2 firmware (e.g. Surface Laptop 8, issue #2) reports boot-time cluster
    // clocks via per-core ~MHz instead of the rated boost, so prefer the
    // rated clock embedded in the name string there, falling back to ~MHz.
    // X1 firmware reports the rated clock accurately via ~MHz already
    // (verified on real X1P-64 hardware), so it keeps the original
    // single-signal path unchanged.
    let signals: [Option<u32>; 2] = match gen {
        Gen::X2 => [id.name_clock_mhz(), id.max_mhz()],
        Gen::X1 => [id.max_mhz(), None],
    };
    signals.into_iter().flatten().find_map(|measured| {
        let picked = pick_by_clock(measured, candidates.clone(), CLOCK_TOLERANCE_MHZ)?;
        let clusters = resolve_clusters(id, cores, picked.clusters);
        Some((ChipProfile { clusters, ..picked }, MatchBasis::Inferred))
    })
}

/// `(name, model, tjmax_c, uarch, lithography, base_ghz, fallback_clusters)`
/// for an as-yet-unconfirmed Snapdragon-X profile — see `family_fallback`.
type FamilyFallbackSpec = (
    &'static str,
    &'static str,
    f64,
    &'static str,
    &'static str,
    f64,
    &'static [(CoreKind, u32)],
);

/// Stage D: Snapdragon X family/subfamily recognized but no SKU candidate
/// matched (an unreleased or otherwise unlisted part). Labels the family
/// honestly and fills in `boost_ghz` from the real measured clock instead of
/// leaving it (and the UI's Boost field) fabricated-looking zero/"—".
fn family_fallback(gen: Gen, sub: Option<Sub>, id: &CpuIdentity) -> (ChipProfile, MatchBasis) {
    let cores = id.logical_cores;
    // Same X2-only rationale as `infer_within_family`: X2 prefers the
    // name-embedded rated clock, X1 keeps its accurate ~MHz reading.
    let boost_mhz = match gen {
        Gen::X2 => id.name_clock_mhz().or_else(|| id.max_mhz()),
        Gen::X1 => id.max_mhz(),
    };
    let boost_ghz = boost_mhz.map(|m| m as f64 / 1000.0).unwrap_or(0.0);
    let (name, model, tjmax_c, uarch, lithography, base_ghz, fallback_clusters): FamilyFallbackSpec = match (gen, sub) {
        (Gen::X2, Some(Sub::Extreme)) => (
            "Snapdragon X2 Elite Extreme",
            "Unknown X2 Elite Extreme SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            heuristic_pe_clusters(cores),
        ),
        (Gen::X2, Some(Sub::Plus)) => (
            "Snapdragon X2 Plus",
            "Unknown X2 Plus SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            heuristic_pe_clusters(cores),
        ),
        (Gen::X2, _) => (
            "Snapdragon X2 Elite",
            "Unknown X2 Elite SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            heuristic_pe_clusters(cores),
        ),
        (Gen::X1, Some(Sub::Plus)) => (
            "Snapdragon X Plus",
            "Unknown X Plus SKU",
            100.0,
            "Oryon",
            "4 nm",
            0.0,
            homogeneous_cluster(cores),
        ),
        (Gen::X1, Some(Sub::Elite)) | (Gen::X1, Some(Sub::Extreme)) => (
            "Snapdragon X Elite",
            "Unknown X Elite SKU",
            100.0,
            "Oryon",
            "4 nm",
            0.0,
            homogeneous_cluster(cores),
        ),
        (Gen::X1, None) => (
            "Snapdragon X",
            "Unknown Snapdragon X SKU",
            100.0,
            "Oryon",
            "4 nm",
            0.0,
            homogeneous_cluster(cores),
        ),
    };
    let clusters = resolve_clusters(id, cores, fallback_clusters);
    let core_naming = match gen {
        Gen::X1 => CoreNaming::UniformPerformance,
        Gen::X2 => CoreNaming::PrimePerformance,
    };
    let profile = ChipProfile {
        id: "FAM",
        name,
        model_match: "",
        model,
        tjmax_c,
        base_ghz,
        boost_ghz,
        tdp_w: 0,
        clusters,
        uarch,
        lithography,
        core_naming,
    };
    (profile, MatchBasis::FamilyOnly)
}

/// Stage E: a known non-X-series vendor/family, but no specific SKU token
/// matched. Never invents a SKU or spec — labels the vendor/family only,
/// with `boost_ghz` from the real measured clock. `None` if the vendor/name
/// doesn't match any recognized non-X-series family (falls through to the
/// last-resort Stage F).
fn vendor_family_fallback(id: &CpuIdentity) -> Option<Detection> {
    let normalized = id.normalized_name();
    let vendor_lower = id.vendor.as_deref().unwrap_or("").to_lowercase();
    let cores = id.logical_cores;
    let boost_ghz = id.max_mhz().map(|m| m as f64 / 1000.0).unwrap_or(0.0);

    let (name, model, core_naming): (&'static str, &'static str, CoreNaming) =
        if normalized.contains("broadcom") || vendor_lower.contains("broadcom") {
            ("Broadcom", "Unknown Broadcom SKU", CoreNaming::UniformPerformance)
        } else if normalized.contains("mediatek") || vendor_lower.contains("mediatek") {
            ("MediaTek", "Unknown MediaTek SKU", CoreNaming::PerformanceEfficiency)
        } else if normalized.contains("nvidia") || vendor_lower.contains("nvidia") {
            ("NVIDIA", "Unknown NVIDIA SKU", CoreNaming::PerformanceEfficiency)
        } else if normalized.contains("qualcomm") || vendor_lower.contains("qualcomm") {
            ("Snapdragon (legacy)", "Unknown Snapdragon SKU", CoreNaming::PerformanceEfficiency)
        } else {
            return None;
        };

    let fallback_clusters = match core_naming {
        CoreNaming::UniformPerformance => homogeneous_cluster(cores),
        _ => heuristic_pe_clusters(cores),
    };
    let clusters = resolve_clusters(id, cores, fallback_clusters);
    Some(Detection {
        profile: ChipProfile {
            id: "VEND",
            name,
            model_match: "",
            model,
            tjmax_c: 100.0,
            base_ghz: 0.0,
            boost_ghz,
            tdp_w: 0,
            clusters,
            uarch: "",
            lithography: "",
            core_naming,
        },
        basis: MatchBasis::FamilyOnly,
    })
}

/// Stage F: nothing recognized at all. Uses the real `VendorIdentifier` when
/// present (e.g. "Broadcom CPU") rather than ever claiming "Snapdragon" for
/// a chip that isn't one. Even P/E split when there are many cores; all-P
/// otherwise (an honest guess, not an assertion this chip has no efficiency
/// tier — hence `PerformanceEfficiency`, the safe default for the unknown).
fn generic_fallback(id: &CpuIdentity) -> Detection {
    let cores = id.logical_cores;
    let (fallback_clusters, tjmax): (&'static [(CoreKind, u32)], f64) = if cores >= 12 {
        (heuristic_pe_clusters(cores), 105.0)
    } else {
        (homogeneous_cluster(cores), 100.0)
    };
    let clusters = resolve_clusters(id, cores, fallback_clusters);
    let name: &'static str = match id.vendor.as_deref() {
        Some(v) if !v.trim().is_empty() => Box::leak(format!("{} CPU", v.trim()).into_boxed_str()),
        _ => "ARM64 CPU",
    };
    Detection {
        profile: ChipProfile {
            id: "GEN",
            name,
            model_match: "",
            model: "Unknown",
            tjmax_c: tjmax,
            base_ghz: 0.0,
            boost_ghz: 0.0,
            tdp_w: 0,
            clusters,
            uarch: "",
            lithography: "",
            core_naming: CoreNaming::PerformanceEfficiency,
        },
        basis: MatchBasis::Generic,
    }
}

/// Match the detected CPU identity against the profile table. Always returns
/// something usable — see the module doc comment for the stage order.
///
/// Real OS topology (`resolve_clusters`) overrides every matched profile's
/// static `clusters`, not just the inferred/fallback ones — a token match
/// (Stage A) still gets the real per-core P/E split when the OS reports one
/// that's consistent with the detected core count, so a diagnostic report's
/// "Cores" line never contradicts its own OS-topology line.
pub fn match_profile(id: &CpuIdentity) -> Detection {
    let normalized = id.normalized_name();
    if !normalized.is_empty() {
        for c in CHIPS {
            if normalized.contains(c.model_match) {
                let clusters = resolve_clusters(id, id.logical_cores, c.clusters);
                return Detection {
                    profile: ChipProfile { clusters, ..c.clone() },
                    basis: MatchBasis::SkuToken,
                };
            }
        }
    }
    if let Some((gen, sub)) = detect_family(id) {
        if let Some((profile, basis)) = infer_within_family(gen, sub, id) {
            return Detection { profile, basis };
        }
        let (profile, basis) = family_fallback(gen, sub, id);
        return Detection { profile, basis };
    }
    if let Some(d) = vendor_family_fallback(id) {
        return d;
    }
    generic_fallback(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(name: &str, cores: u32, perf: Option<u32>, eff: Option<u32>, max_mhz: Option<u32>) -> CpuIdentity {
        CpuIdentity {
            name: Some(name.to_string()),
            identifier: None,
            vendor: Some("Qualcomm Technologies Inc".to_string()),
            per_core_mhz: max_mhz.map(|m| vec![m]).unwrap_or_default(),
            logical_cores: cores,
            perf_cores: perf,
            eff_cores: eff,
        }
    }

    fn identity_with_vendor(name: &str, vendor: &str, cores: u32) -> CpuIdentity {
        CpuIdentity {
            name: Some(name.to_string()),
            identifier: None,
            vendor: Some(vendor.to_string()),
            per_core_mhz: vec![],
            logical_cores: cores,
            perf_cores: None,
            eff_cores: None,
        }
    }

    fn identity_with_per_core_mhz(name: &str, cores: u32, perf: u32, eff: u32, per_core_mhz: Vec<u32>) -> CpuIdentity {
        CpuIdentity {
            name: Some(name.to_string()),
            identifier: Some("ARMv8 (64-bit) Family 8 Model 2 Revision 201".to_string()),
            vendor: Some("Qualcomm Technologies Inc".to_string()),
            per_core_mhz,
            logical_cores: cores,
            perf_cores: Some(perf),
            eff_cores: Some(eff),
        }
    }

    // --- Core-tier vocabulary ---------------------------------------------

    #[test]
    fn tier_badge_uniform_performance_is_p_for_both_kinds() {
        let p = chip_by_id("X126");
        assert_eq!(p.tier_badge(CoreKind::Performance), ("P", "Performance core"));
        assert_eq!(p.tier_badge(CoreKind::Efficiency), ("P", "Performance core"));
    }

    #[test]
    fn tier_badge_prime_performance_is_p_and_p2() {
        let p = chip_by_id("X2E78");
        assert_eq!(p.tier_badge(CoreKind::Performance), ("P", "Prime core"));
        assert_eq!(p.tier_badge(CoreKind::Efficiency), ("P2", "Performance core"));
    }

    #[test]
    fn tier_badge_performance_efficiency_is_p_and_e() {
        let p = chip_by_id("SDM850");
        assert_eq!(p.tier_badge(CoreKind::Performance), ("P", "Performance core"));
        assert_eq!(p.tier_badge(CoreKind::Efficiency), ("E", "Efficiency core"));
    }

    // --- The actual reported bug: issue #2 round 2 ----------------------

    #[test]
    fn x2_elite_78_bare_marketing_name_infers_via_clock() {
        // Surface Laptop 8's real registry string per issue #2: no SKU
        // token, just the marketing name. HWiNFO/CPU-Z read the same 4032
        // MHz rated clock via ~MHz that this app now also reads.
        let id = identity("Qualcomm Snapdragon X2 Elite", 12, Some(6), Some(6), Some(4032));
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-78-100");
        assert_eq!(d.profile.name, "Snapdragon X2 Elite");
        assert_eq!(d.basis, MatchBasis::Inferred);
        assert_eq!(d.profile.total_cores(), 12);
        assert_eq!(d.profile.boost_ghz, 4.0);
        assert_ne!(d.profile.model, "Unknown X2 Elite SKU");
        assert_eq!(d.profile.core_naming, CoreNaming::PrimePerformance);
    }

    #[test]
    fn x2_elite_80_or_84_ambiguous_gets_honest_combined_label() {
        let id = identity("Qualcomm Snapdragon X2 Elite", 12, Some(6), Some(6), Some(4700));
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-80/84-100");
        assert_eq!(d.basis, MatchBasis::Inferred);
        assert_eq!(d.profile.boost_ghz, 4.7);
        assert_eq!(d.profile.base_ghz, 4.0);
    }

    // --- Issue #2 round 3: boot-time cluster clocks in ~MHz mislead
    //     inference; the rated clock embedded in ProcessorNameString rescues
    //     it. -------------------------------------------------------------

    #[test]
    fn x2_elite_78_boot_clocks_in_mhz_infers_via_name_clock() {
        // lexcyn's exact reported identity (Surface Laptop 8): the
        // per-core ~MHz values are boot-time cluster clocks (3072/3437), not
        // the rated boost, so max_mhz() alone misses every candidate. The
        // name's "@ 4.03 GHz" is the real rated clock and must resolve this.
        let id = identity_with_per_core_mhz(
            "Snapdragon X2 Elite @ 4.03 GHz",
            12,
            6,
            6,
            vec![3072, 3072, 3072, 3072, 3072, 3072, 3437, 3437, 3437, 3437, 3437, 3437],
        );
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-78-100");
        assert_eq!(d.basis, MatchBasis::Inferred);
        assert_eq!(d.profile.boost_ghz, 4.0);
        assert_eq!(d.profile.clusters, &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)]);
    }

    #[test]
    fn x2_elite_80_84_boot_clocks_infers_via_name_clock() {
        let id = identity_with_per_core_mhz(
            "Snapdragon X2 Elite @ 4.70 GHz",
            12,
            6,
            6,
            vec![3072, 3072, 3072, 3072, 3072, 3072, 3437, 3437, 3437, 3437, 3437, 3437],
        );
        assert_eq!(match_profile(&id).profile.model, "X2E-80/84-100");
    }

    #[test]
    fn x2_elite_90_18core_boot_clocks_infers_via_name_clock() {
        let id = identity_with_per_core_mhz("Snapdragon X2 Elite @ 5.00 GHz", 18, 12, 6, vec![3072; 18]);
        assert_eq!(match_profile(&id).profile.model, "X2E-90-100");
    }

    #[test]
    fn no_name_clock_and_boot_clock_only_still_honestly_unknown() {
        // Regression: without the name-embedded clock, boot-time cluster
        // clocks correctly fail to infer a SKU — no change from today's
        // honest "Unknown" behavior.
        let id = identity_with_per_core_mhz(
            "Snapdragon X2 Elite",
            12,
            6,
            6,
            vec![3072, 3072, 3072, 3072, 3072, 3072, 3437, 3437, 3437, 3437, 3437, 3437],
        );
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "Unknown X2 Elite SKU");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
    }

    #[test]
    fn x1_token_match_unaffected_by_trailing_name_clock() {
        // Stage A (SKU token) must still win before any clock logic runs.
        let id = identity(
            "Snapdragon(R) X Elite - X1E-78-100 - Qualcomm(R) Oryon(TM) CPU @ 3.40 GHz",
            12,
            None,
            None,
            None,
        );
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X1E-78-100");
        assert_eq!(d.basis, MatchBasis::SkuToken);
    }

    #[test]
    fn name_clock_miss_falls_back_to_measured_mhz() {
        // Name embeds a clock that misses every 12-core X2 Elite candidate
        // (4400 is >180 MHz from both 4000 and 4700); the measured ~MHz
        // rated boost still rescues it, proving the two-signal try-both
        // policy tries name first but doesn't give up when it misses.
        let id = identity("Snapdragon X2 Elite @ 4.40 GHz", 12, Some(6), Some(6), Some(4700));
        assert_eq!(match_profile(&id).profile.model, "X2E-80/84-100");
    }

    #[test]
    fn x1_inference_ignores_name_clock_uses_measured_mhz_only() {
        // The X2-only constraint: X1 firmware reports the rated clock
        // accurately via ~MHz (verified on real X1P-64 hardware), so a
        // name-embedded clock must NOT be consulted for X1. Here the name
        // says "@ 4.00 GHz" (which would pick X1E-80-100 if read), but the
        // measured ~MHz is 3400 (X1E-78-100's rated clock) — the correct
        // result proves the name clock was ignored, not merely deprioritized.
        let id = identity("Snapdragon X1 Elite @ 4.00 GHz", 12, None, None, Some(3400));
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X1E-78-100");
        assert_ne!(d.profile.model, "X1E-80-100");
    }

    #[test]
    fn token_matched_profile_still_honors_real_topology_over_static_table() {
        // Real report from this dev machine (X1P-64-100, issue #2 follow-up):
        // the static table assumes a homogeneous 10-Performance-core cluster,
        // but the OS topology reports an asymmetric 6P/4E split (confirmed by
        // per-core ~MHz: 6 cores at 3418 MHz, 4 at 2976 MHz). The matched
        // profile's `clusters` must reflect that real split, not the stale
        // static one — otherwise a diagnostics dump's "Cores" line
        // contradicts its own OS-topology line.
        let id = identity(
            "Snapdragon(R) X 10-core X1P64100 @ 3.40 GHz",
            10,
            Some(6),
            Some(4),
            Some(3418),
        );
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X1P-64-100");
        assert_eq!(d.basis, MatchBasis::SkuToken);
        assert_eq!(d.profile.clusters, &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 4)]);
        // X1 is uniform: even though topology reports a 6/4 split, the badge
        // vocabulary must still be all-"P" (no efficiency cores on X1).
        assert_eq!(d.profile.tier_badge(CoreKind::Efficiency), ("P", "Performance core"));
    }

    #[test]
    fn x2_elite_extreme_bare_name_infers_18_core_skus() {
        let id96 = identity("Snapdragon X2 Elite Extreme", 18, Some(12), Some(6), Some(5000));
        assert_eq!(match_profile(&id96).profile.model, "X2E-96-100");
        let id94 = identity("Snapdragon X2 Elite Extreme", 18, Some(12), Some(6), Some(4700));
        assert_eq!(match_profile(&id94).profile.model, "X2E-94-100");
    }

    #[test]
    fn x2_elite_bare_name_infers_18_core_skus() {
        let id90 = identity("Snapdragon X2 Elite", 18, Some(12), Some(6), Some(5000));
        assert_eq!(match_profile(&id90).profile.model, "X2E-90-100");
        let id88 = identity("Snapdragon X2 Elite", 18, Some(12), Some(6), Some(4700));
        assert_eq!(match_profile(&id88).profile.model, "X2E-88-100");
    }

    #[test]
    fn name_independent_path_uses_identifier_and_vendor() {
        // Name totally unusable (empty/garbage); Identifier says Oryon V3
        // (Model 2) and vendor says Qualcomm — still infers correctly.
        let id = CpuIdentity {
            name: Some("Unknown CPU".to_string()),
            identifier: Some("ARMv8 (64-bit) Family 8 Model 2 Revision 201".to_string()),
            vendor: Some("Qualcomm Technologies Inc".to_string()),
            per_core_mhz: vec![4032],
            logical_cores: 12,
            perf_cores: Some(6),
            eff_cores: Some(6),
        };
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-78-100");
    }

    #[test]
    fn non_qualcomm_vendor_with_matching_model_number_is_not_mislabeled() {
        let id = CpuIdentity {
            name: Some("Unknown CPU".to_string()),
            identifier: Some("ARMv8 (64-bit) Family 8 Model 2 Revision 201".to_string()),
            vendor: Some("Some Other Vendor Inc".to_string()),
            per_core_mhz: vec![4032],
            logical_cores: 12,
            perf_cores: Some(6),
            eff_cores: Some(6),
        };
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "Unknown");
        assert_eq!(d.basis, MatchBasis::Generic);
        assert_eq!(d.profile.name, "Some Other Vendor Inc CPU");
    }

    #[test]
    fn unknown_future_x2_sku_gets_family_fallback_with_real_boost() {
        // A hypothetical future 24-core X2 Elite: no clock candidate fits.
        let id = identity("Snapdragon X2 Elite", 24, Some(18), Some(6), Some(5200));
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "Snapdragon X2 Elite");
        assert_eq!(d.profile.model, "Unknown X2 Elite SKU");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
        assert_eq!(d.profile.boost_ghz, 5.2); // real measured clock, not 0.0/"—"
        assert_eq!(d.profile.total_cores(), 24);
        assert_eq!(d.profile.clusters, &[(CoreKind::Performance, 18), (CoreKind::Efficiency, 6)]);
    }

    // --- Token match (Stage A) regressions --------------------------------

    #[test]
    fn x2_elite_78_real_world_token_format_matches() {
        let id = identity(
            "Snapdragon(R) X2 Elite - X2E78100 - Qualcomm(R) Oryon(TM) CPU",
            12,
            None,
            None,
            None,
        );
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-78-100");
        assert_eq!(d.basis, MatchBasis::SkuToken);
        assert_eq!(d.profile.clusters, &[(CoreKind::Performance, 6), (CoreKind::Efficiency, 6)]);
    }

    #[test]
    fn x2_elite_88_18core_token_matches() {
        let id = identity("Snapdragon X2 Elite - X2E88100", 18, None, None, None);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X2E-88-100");
        assert_eq!(d.profile.total_cores(), 18);
    }

    #[test]
    fn x2_plus_64_10core_token_matches() {
        let id = identity("Snapdragon X2 Plus - X2P64100", 10, None, None, None);
        assert_eq!(match_profile(&id).profile.model, "X2P-64-100");
    }

    #[test]
    fn x1e_token_matches_the_exact_sku_not_the_old_catch_all() {
        // Regression: the old `model_match: "x1e"` matched EVERY X Elite SKU
        // to X1E-80-100. Each SKU must now resolve to its own model.
        let id78 = identity("Snapdragon(R) X Elite - X1E78100", 12, None, None, None);
        assert_eq!(match_profile(&id78).profile.model, "X1E-78-100");
        let id84 = identity("Snapdragon(R) X Elite - X1E84100", 12, None, None, None);
        assert_eq!(match_profile(&id84).profile.model, "X1E-84-100");
        let id80 = identity("Snapdragon(R) X Elite - X1E80100", 12, None, None, None);
        assert_eq!(match_profile(&id80).profile.model, "X1E-80-100");
    }

    #[test]
    fn x1_plus_64_regression() {
        let id = identity("Snapdragon(R) X Plus - X1P64100", 10, None, None, None);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X1P-64-100");
        assert_eq!(d.profile.name, "Snapdragon X Plus");
    }

    #[test]
    fn x1_26_regression_guards_key_change() {
        let id = identity("Snapdragon X - X1-26-100", 8, None, None, None);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "X1-26-100");
        assert_eq!(d.profile.name, "Snapdragon X");
    }

    #[test]
    fn non_snapdragon_falls_back_to_generic_honest_label() {
        let id = identity_with_vendor("Some Other CPU", "ACME Silicon", 8);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "Unknown");
        assert_eq!(d.profile.name, "ACME Silicon CPU");
        assert_eq!(d.profile.total_cores(), 8);
        assert_eq!(d.basis, MatchBasis::Generic);
    }

    #[test]
    fn no_vendor_at_all_falls_back_to_bare_arm64_cpu() {
        let id = CpuIdentity {
            name: Some("Totally Unknown".to_string()),
            identifier: None,
            vendor: None,
            per_core_mhz: vec![],
            logical_cores: 4,
            perf_cores: None,
            eff_cores: None,
        };
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "ARM64 CPU");
        assert_eq!(d.profile.model, "Unknown");
    }

    // --- Legacy Qualcomm Windows-on-ARM lineup (Stage A, verified strings) -

    #[test]
    fn legacy_8cx_family_ordering_resolves_the_exact_generation() {
        // Ordering hazard: "8cxgen3"/"8cxgen2" must win over bare "8cx",
        // which must win over "8c" — each a substring of the ones above it.
        let gen3 = identity_with_vendor("Snapdragon (TM) 8cx Gen 3 @ 2.69 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&gen3).profile.model, "SC8280XP");
        let gen2 = identity_with_vendor("Snapdragon 8cx Gen 2 @ 3.15 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&gen2).profile.model, "SC8180X+");
        let gen1 = identity_with_vendor("Snapdragon 8cx @ 2.84 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&gen1).profile.model, "SC8180X");
        let plain8c = identity_with_vendor("Snapdragon 8c @ 2.45 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&plain8c).profile.model, "SC8180");
    }

    #[test]
    fn legacy_7c_family_ordering_resolves_the_exact_generation() {
        // Qualcomm's own marketing writes "7c+ Gen 3" with a literal plus
        // sign (mirrored verbatim by OEM firmware elsewhere, e.g. X2's
        // literal hyphens) — normalizes to "7cgen3", matching our token.
        // (PassMark's DB renders this chip as "7cPlus Gen 3" — its own
        // display sanitization of the "+", not necessarily the raw OS
        // string; if some firmware DOES spell out "Plus", it falls through
        // to the bare "7c" entry below rather than mismatching family.)
        let gen3 = identity_with_vendor("Snapdragon 7c+ Gen 3 @ 2.40 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&gen3).profile.model, "SC7280");
        let gen2 = identity_with_vendor("Snapdragon 7c Gen 2 @ 2.55 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&gen2).profile.model, "SC7180P");
        let plain7c = identity_with_vendor("Snapdragon 7c @ 2.40 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&plain7c).profile.model, "SC7180");
    }

    #[test]
    fn legacy_microsoft_sq_lineup_matches() {
        let sq1 = identity_with_vendor("Microsoft SQ1 @ 3.0 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&sq1).profile.model, "SQ1");
        let sq2 = identity_with_vendor("Microsoft SQ2 @ 3.15 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&sq2).profile.model, "SQ2");
        let sq3 = identity_with_vendor("Microsoft SQ3 @ 3.0 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&sq3).profile.model, "SQ3");
    }

    #[test]
    fn legacy_835_and_850_match_and_use_pe_vocabulary() {
        let s850 = identity_with_vendor("Snapdragon 850 @ 2.96 GHz", "Qualcomm Technologies Inc", 8);
        let d850 = match_profile(&s850);
        assert_eq!(d850.profile.model, "SDM850");
        assert_eq!(d850.basis, MatchBasis::SkuToken);
        assert_eq!(d850.profile.tier_badge(CoreKind::Efficiency), ("E", "Efficiency core"));
        let s835 = identity_with_vendor("Snapdragon 835 @ 2.45 GHz", "Qualcomm Technologies Inc", 8);
        assert_eq!(match_profile(&s835).profile.model, "MSM8998");
    }

    #[test]
    fn unanticipated_legacy_format_is_guarded_from_x1_misfire() {
        // No Stage A token matches (an adjective splits "Snapdragon" from
        // "850"), but the guard tokens still catch it and route to the
        // honest legacy-Qualcomm fallback rather than misreading an 8-core
        // 2.96 GHz legacy chip as the Snapdragon X 8-core (3.0 GHz — only 40
        // MHz off, inside the clock-inference tolerance).
        let id = identity_with_vendor("Snapdragon Turbo 850 Special Edition", "Qualcomm Technologies Inc", 8);
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "Snapdragon (legacy)");
        assert_ne!(d.profile.model, "X1-26-100");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
    }

    // --- Raspberry Pi ------------------------------------------------------

    #[test]
    fn raspberry_pi4_matches_and_is_uniform_performance() {
        let id = identity_with_vendor("BCM2711 (ARM Cortex-A72) 1.50 GHz", "Broadcom", 4);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "BCM2711");
        assert_eq!(d.profile.name, "Broadcom BCM2711");
        assert_eq!(d.basis, MatchBasis::SkuToken);
        assert_eq!(d.profile.core_naming, CoreNaming::UniformPerformance);
        assert_eq!(d.profile.tier_badge(CoreKind::Performance), ("P", "Performance core"));
    }

    #[test]
    fn unknown_broadcom_board_gets_honest_vendor_fallback() {
        let id = identity_with_vendor("Unknown SoC", "Broadcom", 4);
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "Broadcom");
        assert_eq!(d.profile.model, "Unknown Broadcom SKU");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
    }

    #[test]
    fn unknown_mediatek_gets_honest_vendor_fallback() {
        let id = identity_with_vendor("MediaTek Kompanio Something", "MediaTek", 8);
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "MediaTek");
        assert_eq!(d.profile.model, "Unknown MediaTek SKU");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
    }

    #[test]
    fn unknown_nvidia_gets_honest_vendor_fallback() {
        let id = identity_with_vendor("NVIDIA N1X", "NVIDIA", 20);
        let d = match_profile(&id);
        assert_eq!(d.profile.name, "NVIDIA");
        assert_eq!(d.profile.model, "Unknown NVIDIA SKU");
        assert_eq!(d.basis, MatchBasis::FamilyOnly);
    }
}
