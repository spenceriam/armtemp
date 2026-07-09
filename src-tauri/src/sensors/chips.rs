//! Chip profiles for the Snapdragon X / X2 families. This is the real silicon
//! reference table ported from the Claude design's `chips` array. It is used to
//! LABEL detected hardware (name, model, TjMax, cluster layout for P/E core
//! coloring) — never to fabricate readings.
//!
//! Detection is layered, cheapest/most-certain evidence first (see
//! `match_profile`):
//!   A. SKU token in `ProcessorNameString` (e.g. "X2E78100") — exact, when present.
//!   B. Family/subfamily parsed from the name ("X2 Elite", "X2 Elite Extreme",
//!      "X2 Plus", …), or — if the name carries no usable text at all — the
//!      registry `Identifier`'s MIDR part number (Oryon generation) plus vendor.
//!   C. Within that family, the exact SKU inferred from real, hardware-reported
//!      signals: logical core count, P/E topology, and each core's `~MHz`
//!      (rated/boost clock) — the same signals CPU-Z and HWiNFO use, since
//!      issue #2 showed some OEM firmware (Surface) reports a bare marketing
//!      name with no SKU token in it at all.
//!   D. Family recognized but no SKU matches (unreleased/future part): label
//!      the family honestly, with boost populated from the real measured
//!      clock — never "Generic".
//!   E. Nothing Snapdragon-shaped detected: generic fallback.
//!
//! The actual CPU identity is gathered in `sensors::identity` (registry +
//! `GetSystemInfo`/topology query in `sensors::pdh`).

use crate::sensors::identity::CpuIdentity;
use crate::sensors::types::CoreKind;

/// A known Snapdragon X-family SoC profile.
#[derive(Clone, Debug)]
pub struct ChipProfile {
    pub clusters: &'static [(CoreKind, u32)],
    pub id: &'static str,
    pub name: &'static str,
    /// Alphanumeric-lowercase substring matched against the normalized
    /// detected CPU name (see `CpuIdentity::normalized_name`). Empty for
    /// profiles that are never reached via name-token matching (Stage B/C/D
    /// results).
    pub model_match: &'static str,
    /// Marketing model string.
    pub model: &'static str,
    /// Thermal junction max in °C.
    pub tjmax_c: f64,
    /// Base (all-core) clock GHz (informational; not always real).
    pub base_ghz: f64,
    /// Boost clock GHz (informational).
    pub boost_ghz: f64,
    /// Nominal TDP in watts (informational). 0 means unpublished/unverified —
    /// the UI renders that as an honest "—" (see `pdh.rs`'s `tdp_w` mapping).
    pub tdp_w: u32,
    /// Microarchitecture label, e.g. "Oryon" (X1 family) or "Oryon 3" (X2
    /// family, Qualcomm's 3rd Gen Oryon). Empty string for the generic
    /// fallback profile.
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

/// How a `ChipProfile` was determined — surfaced to the UI/diagnostics report
/// so "we're not sure" is never presented as "we're sure".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchBasis {
    /// An exact SKU token (e.g. "x2e78") was found in the CPU name.
    SkuToken,
    /// No SKU token in the name; the exact SKU was inferred from real core
    /// count + real rated clock within a family identified from the name (or
    /// Identifier).
    Inferred,
    /// The family/subfamily is known but no specific SKU matched (unreleased
    /// part, or two SKUs are indistinguishable from CPU-visible signals).
    FamilyOnly,
    /// Nothing Snapdragon-shaped was detected at all.
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

/// The real Snapdragon X / X2 lineup (matches the design and real silicon).
/// X1 specs are sourced from Qualcomm's X Elite / X Plus product briefs and
/// notebookcheck's per-SKU pages. X2 specs are sourced from Qualcomm's X2
/// Elite / X2 Plus product briefs. Qualcomm does not publish a TDP for every
/// SKU (OEM-defined power limits vary widely); `tdp_w` is 0 (renders as "—")
/// wherever no published figure was found — never a guess.
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

/// Stage B: family + subfamily from the CPU name; if the name carries no
/// usable text at all, fall back to the registry `Identifier`'s MIDR part
/// number (Oryon generation) — gated on the vendor string actually saying
/// Qualcomm, so a non-Snapdragon ARM chip that happens to share a Model
/// number never gets mislabeled.
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
        if normalized.contains("x1") || normalized.contains("snapdragon") {
            let sub = if normalized.contains("elite") {
                Some(Sub::Elite)
            } else if normalized.contains("plus") {
                Some(Sub::Plus)
            } else {
                None
            };
            return Some((Gen::X1, sub));
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

/// Stage C: within a known family/subfamily, infer the exact SKU from real
/// core count + real rated clock. `None` if the family has no candidates for
/// this core count, or the measured clock is too far from every candidate.
fn infer_within_family(gen: Gen, sub: Option<Sub>, id: &CpuIdentity) -> Option<(ChipProfile, MatchBasis)> {
    let cores = id.logical_cores;
    let max_mhz = id.max_mhz()?;
    let candidates = match (gen, sub) {
        (Gen::X2, Some(Sub::Extreme)) => x2_elite_extreme_candidates(cores),
        (Gen::X2, Some(Sub::Plus)) => x2_plus_candidates(cores),
        (Gen::X2, Some(Sub::Elite)) | (Gen::X2, None) => x2_elite_candidates(cores),
        (Gen::X1, Some(Sub::Elite)) => x1_elite_candidates(cores),
        (Gen::X1, Some(Sub::Plus)) => x1_plus_candidates(cores),
        (Gen::X1, Some(Sub::Extreme)) => vec![], // no X1 "Extreme" tier exists
        (Gen::X1, None) => x1_candidates(cores),
    };
    let picked = pick_by_clock(max_mhz, candidates, CLOCK_TOLERANCE_MHZ)?;
    let clusters = resolve_clusters(id, cores, picked.clusters);
    Some((ChipProfile { clusters, ..picked }, MatchBasis::Inferred))
}

/// `(name, model, tjmax_c, uarch, lithography, base_ghz, fallback_clusters)`
/// for an as-yet-unconfirmed profile — see `family_fallback`.
type FamilyFallbackSpec = (
    &'static str,
    &'static str,
    f64,
    &'static str,
    &'static str,
    f64,
    &'static [(CoreKind, u32)],
);

/// Stage D: family/subfamily recognized but no SKU candidate matched (an
/// unreleased or otherwise unlisted part). Labels the family honestly and
/// fills in `boost_ghz` from the real measured clock instead of leaving it
/// (and the UI's Boost field) fabricated-looking zero/"—".
fn family_fallback(gen: Gen, sub: Option<Sub>, id: &CpuIdentity) -> (ChipProfile, MatchBasis) {
    let cores = id.logical_cores;
    let boost_ghz = id.max_mhz().map(|m| m as f64 / 1000.0).unwrap_or(0.0);
    let (name, model, tjmax_c, uarch, lithography, base_ghz, fallback_clusters): FamilyFallbackSpec = match (gen, sub) {
        (Gen::X2, Some(Sub::Extreme)) => (
            "Snapdragon X2 Elite Extreme",
            "Unknown X2 Elite Extreme SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            x2_family_clusters(cores),
        ),
        (Gen::X2, Some(Sub::Plus)) => (
            "Snapdragon X2 Plus",
            "Unknown X2 Plus SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            x2_family_clusters(cores),
        ),
        (Gen::X2, _) => (
            "Snapdragon X2 Elite",
            "Unknown X2 Elite SKU",
            105.0,
            "Oryon 3",
            "3 nm",
            0.0,
            x2_family_clusters(cores),
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
    };
    (profile, MatchBasis::FamilyOnly)
}

/// Stage E: nothing Snapdragon-shaped detected. Even P/E split when there
/// are many cores; all-P otherwise.
fn generic_fallback(id: &CpuIdentity) -> Detection {
    let cores = id.logical_cores;
    let (fallback_clusters, tjmax): (&'static [(CoreKind, u32)], f64) = if cores >= 12 {
        (
            Box::leak(
                vec![
                    (CoreKind::Performance, cores * 2 / 3),
                    (CoreKind::Efficiency, cores - cores * 2 / 3),
                ]
                .into_boxed_slice(),
            ),
            105.0,
        )
    } else {
        (homogeneous_cluster(cores), 100.0)
    };
    let clusters = resolve_clusters(id, cores, fallback_clusters);
    Detection {
        profile: ChipProfile {
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
/// "Cores" line never contradicts its own "Real P/E topology" line.
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

    #[test]
    fn token_matched_profile_still_honors_real_topology_over_static_table() {
        // Real report from this dev machine (X1P-64-100, issue #2 follow-up):
        // the static table assumes a homogeneous 10-Performance-core cluster,
        // but the OS topology reports an asymmetric 6P/4E split (confirmed by
        // per-core ~MHz: 6 cores at 3418 MHz, 4 at 2976 MHz). The matched
        // profile's `clusters` must reflect that real split, not the stale
        // static one — otherwise a diagnostics dump's "Cores" line
        // contradicts its own "Real P/E topology" line.
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
        assert_eq!(d.profile.model, "Generic");
        assert_eq!(d.basis, MatchBasis::Generic);
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
    fn non_snapdragon_falls_back_to_generic_even_split() {
        let id = identity("Some Other CPU", 8, None, None, None);
        let d = match_profile(&id);
        assert_eq!(d.profile.model, "Generic");
        assert_eq!(d.profile.name, "Snapdragon");
        assert_eq!(d.profile.total_cores(), 8);
        assert_eq!(d.basis, MatchBasis::Generic);
    }
}
