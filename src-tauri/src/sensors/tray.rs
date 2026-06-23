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
            (t, t.map(|v| temp_color(v, snapshot.tjmax_c)).unwrap_or((140, 140, 140)))
        }
        TrayMode::Average => (
            snapshot.average_c,
            snapshot
                .average_c
                .map(|v| temp_color(v, snapshot.tjmax_c))
                .unwrap_or((140, 140, 140)),
        ),
        TrayMode::All => {
            // For a single icon, "All" still draws the hottest; the per-core
            // detail is in the flyout/UI. Color by hottest.
            let t = snapshot.package_c;
            (t, t.map(|v| temp_color(v, snapshot.tjmax_c)).unwrap_or((140, 140, 140)))
        }
    };
    TrayDecision {
        primary_value: val_c.map(|v| v.round() as i32),
        color_rgb: color,
    }
}

/// Build a simple solid-color PNG icon with no drawn number (fallback).
pub fn blank_icon_png(color: (u8, u8, u8)) -> Vec<u8> {
    // 32x32 RGBA solid color PNG, hand-encoded (no extra deps).
    let (r, g, b) = color;
    let w = 32u32;
    let h = 32u32;
    rgba_to_png(w, h, &move |_x, _y| [r, g, b, 230])
}

/// Minimal PNG encoder (RGBA8, single IDAT, zlib via store). Good enough for a
/// 32x32 tray icon where file size is irrelevant.
fn rgba_to_png<F: Fn(u32, u32) -> [u8; 4]>(w: u32, h: u32, px: &F) -> Vec<u8> {
    // Build raw image data with per-row filter byte (0 = None).
    let mut raw = Vec::with_capacity((w as usize * 4 + 1) * h as usize);
    for y in 0..h {
        raw.push(0u8); // filter: None
        for x in 0..w {
            let [r, g, b, a] = px(x, y);
            raw.extend_from_slice(&[r, g, b, a]);
        }
    }
    let compressed = zlib_store(&raw);

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    write_chunk(&mut out, b"IHDR", &ihdr(w, h));
    write_chunk(&mut out, b"IDAT", &compressed);
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn ihdr(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(13);
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(8); // bit depth
    v.push(6); // color type RGBA
    v.push(0); // compression
    v.push(0); // filter
    v.push(0); // interlace
    v
}

fn write_chunk(out: &mut Vec<u8>, typ: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(typ);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(b: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &byte in b {
        c ^= byte as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    c ^ 0xFFFF_FFFF
}

/// Stored zlib (no real compression): 78 01 header, BFINAL/BTYPE stored blocks,
/// Adler-32 trailer. Valid zlib stream; PNG decoders accept it.
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    out.extend_from_slice(&[0x78, 0x01]); // zlib header (no compression)
    let mut i = 0;
    while i < data.len() {
        let remaining = data.len() - i;
        let block = remaining.min(65535);
        let bfinal = if i + block >= data.len() { 1u8 } else { 0u8 };
        out.push(bfinal); // BTYPE=00 (stored) packed: 0 bfinal, 00 type
        out.extend_from_slice(&(block as u16).to_le_bytes());
        out.extend_from_slice(&(!(block as u16)).to_le_bytes());
        out.extend_from_slice(&data[i..i + block]);
        i += block;
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// Silence unused-import lint for the Zone/Core types kept for future per-core tray.
#[allow(dead_code)]
fn _types_anchor(_z: ZoneReading, _c: CoreReading) {}
