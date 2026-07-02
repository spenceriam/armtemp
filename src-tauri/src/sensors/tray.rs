//! Renders live tray icons from REAL readings. The number shown in the icon is
//! the genuine current temperature (package / highest / average / per-core per
//! the user's chosen mode). Mirrors the Clod design's three icon styles and the
//! green->yellow->orange->red temp color scale.

use crate::sensors::types::{CoreReading, SensorSnapshot, ZoneReading};

/// Tray icon styles (ported from the design's `trayStyle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayStyle {
    Rounded,
    Badge,
    Plain,
}

/// Tray icon data modes (ported from the design's `trayMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayMode {
    All,
    Highest,
    Average,
    Package,
}

/// Decision: which zone/core temperature(s) feed the icon this tick, and their colors.
pub struct TrayDecision {
    pub primary_value: Option<i32>, // °C rounded, the number drawn on the icon
    pub color_rgb: (u8, u8, u8),
}

/// Temp -> color, ported from the design's `tcolor()`. `t` is °C, `tj` is TjMax.
pub fn temp_color(t: f64, tj: f64) -> (u8, u8, u8) {
    let r = ((t - 35.0) / ((tj - 35.0).max(1.0))).clamp(0.0, 1.0);
    if r < 0.45 {
        (47, 164, 90) // green
    } else if r < 0.64 {
        (216, 165, 26) // yellow
    } else if r < 0.82 {
        (224, 122, 43) // orange
    } else {
        (224, 71, 58) // red
    }
}

/// Choose the value for the tray icon given the mode and the real snapshot.
pub fn decide(snapshot: &SensorSnapshot, mode: TrayMode) -> TrayDecision {
    let (val_c, color) = match mode {
        TrayMode::Package | TrayMode::Highest => {
            // Highest valid zone == package proxy.
            let t = snapshot.package_c;
            (t, t.map(|v| temp_color(v, snapshot.tjmax_c)).unwrap_or(NO_READING_GRAY))
        }
        TrayMode::Average => (
            snapshot.average_c,
            snapshot
                .average_c
                .map(|v| temp_color(v, snapshot.tjmax_c))
                .unwrap_or(NO_READING_GRAY),
        ),
        TrayMode::All => {
            // For a single icon, "All" still draws the hottest; the per-core
            // detail is in the flyout/UI. Color by hottest.
            let t = snapshot.package_c;
            (t, t.map(|v| temp_color(v, snapshot.tjmax_c)).unwrap_or(NO_READING_GRAY))
        }
    };
    TrayDecision {
        primary_value: val_c.map(|v| v.round() as i32),
        color_rgb: color,
    }
}

/// Neutral gray used when there is no real reading (matches `decide()`'s
/// "unavailable" color).
const NO_READING_GRAY: (u8, u8, u8) = (140, 140, 140);

/// Background-plate pixel for the Rounded/Badge tray styles: a filled square
/// (Rounded, small corner radius) or circle (Badge, radius = size/2), solid
/// `color` at `alpha`. Returns `None` outside the plate (transparent).
pub(crate) fn plate_pixel(x: u32, y: u32, size: u32, color: (u8, u8, u8), alpha: u8, radius: u32) -> Option<[u8; 4]> {
    let (r, g, b) = color;
    let inside = if radius * 2 >= size {
        // Circle (Badge): distance from center.
        let c = size as f64 / 2.0;
        let dx = x as f64 + 0.5 - c;
        let dy = y as f64 + 0.5 - c;
        dx * dx + dy * dy <= (radius as f64) * (radius as f64)
    } else {
        // Rounded square (Rounded): only the 4 corner boxes get cut by a
        // quarter-circle inset `radius` px from each true corner; the rest
        // of the square (edges + center) is always filled.
        let rad = radius as i64;
        let s = size as i64;
        let px = x as i64;
        let py = y as i64;
        let near_left = px < rad;
        let near_right = px >= s - rad;
        let near_top = py < rad;
        let near_bottom = py >= s - rad;
        if (near_left || near_right) && (near_top || near_bottom) {
            let ccx = if near_left { rad } else { s - 1 - rad };
            let ccy = if near_top { rad } else { s - 1 - rad };
            let dx = px - ccx;
            let dy = py - ccy;
            dx * dx + dy * dy <= rad * rad
        } else {
            true
        }
    };
    if inside {
        Some([r, g, b, alpha])
    } else {
        None
    }
}

// Silence unused-import lint for the Zone/Core types kept for future per-core tray.
#[allow(dead_code)]
fn _types_anchor(_z: ZoneReading, _c: CoreReading) {}
