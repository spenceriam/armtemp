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

/// 3x5 bitmap digit font. Index 0-9 = digits, index 10 = dash (drawn when
/// there is no real reading — never a fabricated number). Each row byte uses
/// bits 2,1,0 for columns 0,1,2 (1 = filled pixel).
const GLYPHS_3X5: [[u8; 5]; 11] = [
    [7, 5, 5, 5, 7], // 0
    [2, 6, 2, 2, 7], // 1
    [7, 1, 7, 4, 7], // 2
    [7, 1, 7, 1, 7], // 3
    [5, 5, 7, 1, 1], // 4
    [7, 4, 7, 1, 7], // 5
    [7, 4, 7, 5, 7], // 6
    [7, 1, 1, 1, 1], // 7
    [7, 5, 7, 5, 7], // 8
    [7, 5, 7, 1, 7], // 9
    [0, 0, 7, 0, 0], // dash (no reading)
];

fn glyph_for(ch: u8) -> &'static [u8; 5] {
    if ch.is_ascii_digit() {
        &GLYPHS_3X5[(ch - b'0') as usize]
    } else {
        &GLYPHS_3X5[10]
    }
}

/// Neutral gray used when there is no real reading (matches `decide()`'s
/// "unavailable" color).
const NO_READING_GRAY: (u8, u8, u8) = (140, 140, 140);

/// Render the real temperature into a tray icon — Core Temp's signature
/// feature. `value` is already unit-converted and rounded by the caller;
/// `None` draws an honest dash, never a fabricated number. Transparent
/// background so the OS tray/taskbar shows through; `style` adds an optional
/// background plate for contrast. Rendered oversized (`size`, e.g. 48) so
/// Windows' high-DPI tray scaling stays legible.
pub fn number_icon_png(value: Option<i32>, color: (u8, u8, u8), style: TrayStyle, size: u32) -> Vec<u8> {
    let text: alloc_free_string::TinyString = match value {
        Some(v) => alloc_free_string::from_i32(v),
        None => alloc_free_string::dash(),
    };
    let glyphs: Vec<&'static [u8; 5]> = text.as_bytes().iter().map(|&b| glyph_for(b)).collect();
    let n = glyphs.len().max(1) as u32;

    // Glyphs are 3 wide + 1px gap between them; pick the largest integer scale
    // that fits both dimensions with a margin, so 2-digit °C and 3-digit °F
    // both render legibly.
    let unit_w = 3 * n + (n - 1);
    let unit_h = 5u32;
    let margin = 0.82; // leave a small border so the plate doesn't clip
    let scale = ((size as f64 * margin / unit_w as f64).min(size as f64 * margin / unit_h as f64))
        .floor()
        .max(1.0) as u32;

    let block_w = unit_w * scale;
    let block_h = unit_h * scale;
    let ox = (size.saturating_sub(block_w)) / 2;
    let oy = (size.saturating_sub(block_h)) / 2;

    let (r, g, b) = color;
    let digit_color = match style {
        // Rounded/Badge draw a dark or colored plate, so use a light digit
        // for contrast; Plain draws straight on the transparent background.
        TrayStyle::Rounded => (255u8, 255u8, 255u8),
        TrayStyle::Badge => (255u8, 255u8, 255u8),
        TrayStyle::Plain => (r, g, b),
    };

    rgba_to_png(size, size, &move |x, y| {
        // Background plate.
        let plate = match style {
            TrayStyle::Plain => None,
            TrayStyle::Rounded => plate_pixel(x, y, size, (r / 3, g / 3, b / 3), 190, 6),
            TrayStyle::Badge => plate_pixel(x, y, size, (r, g, b), 235, size / 2),
        };

        // Foreground glyph pixel, if inside the glyph block.
        if x >= ox && y >= oy {
            let gx = (x - ox) / scale;
            let gy = (y - oy) / scale;
            if gy < unit_h && gx < unit_w {
                let col_in_glyph = gx % 4; // 3 pixels + 1 gap column
                let glyph_idx = (gx / 4) as usize;
                if col_in_glyph < 3 && glyph_idx < glyphs.len() {
                    let row = glyphs[glyph_idx][gy as usize];
                    let bit = 1u8 << (2 - col_in_glyph);
                    if row & bit != 0 {
                        let (dr, dg, db) = digit_color;
                        return [dr, dg, db, 255];
                    }
                }
            }
        }

        plate.unwrap_or([0, 0, 0, 0])
    })
}

/// Background-plate pixel for the Rounded/Badge tray styles: a filled square
/// (Rounded, small corner radius) or circle (Badge, radius = size/2), solid
/// `color` at `alpha`. Returns `None` outside the plate (transparent).
fn plate_pixel(x: u32, y: u32, size: u32, color: (u8, u8, u8), alpha: u8, radius: u32) -> Option<[u8; 4]> {
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

/// Tiny stack-allocated ASCII string (max 4 bytes: signed 3-digit temps + a
/// dash never exceed this) so the digit renderer avoids a heap `String`.
mod alloc_free_string {
    pub struct TinyString {
        buf: [u8; 4],
        len: u8,
    }
    impl TinyString {
        pub fn as_bytes(&self) -> &[u8] {
            &self.buf[..self.len as usize]
        }
    }
    pub fn dash() -> TinyString {
        TinyString { buf: [b'-', 0, 0, 0], len: 1 }
    }
    pub fn from_i32(v: i32) -> TinyString {
        let mut buf = [0u8; 4];
        let mut len = 0usize;
        let neg = v < 0;
        let mut n = v.unsigned_abs();
        let mut digits = [0u8; 4];
        let mut dlen = 0usize;
        if n == 0 {
            digits[0] = b'0';
            dlen = 1;
        } else {
            while n > 0 && dlen < digits.len() {
                digits[dlen] = b'0' + (n % 10) as u8;
                n /= 10;
                dlen += 1;
            }
        }
        if neg && len < buf.len() {
            buf[len] = b'-';
            len += 1;
        }
        for i in (0..dlen).rev() {
            if len < buf.len() {
                buf[len] = digits[i];
                len += 1;
            }
        }
        TinyString { buf, len: len as u8 }
    }
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
