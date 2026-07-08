//! Native Windows tray-icon digit renderer.
//!
//! Renders the tray/taskbar temperature number using the real system font
//! (Segoe UI, GDI-antialiased) instead of a hand-drawn pixel font, so it
//! reads as an actual number at the notification area's native size.
//!
//! Public interface (`tray_icon_size`, `render_number_rgba`) is the portable
//! seam: a future macOS menu-bar or Linux tray backend implements the same
//! two functions without touching callers in `lib.rs`.

use crate::sensors::tray::{plate_pixel, TrayStyle};
use windows::core::HSTRING;
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW,
    GdiFlush, GetTextExtentPoint32W, SelectObject, SetBkMode, SetTextColor, ANTIALIASED_QUALITY,
    BITMAPINFO, BI_RGB, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS,
    DT_CENTER, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_BOLD, FW_MEDIUM, HFONT, OUT_DEFAULT_PRECIS,
    TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};

/// Native small-icon size for the notification area at the current DPI
/// (typically 16, 20, or 24 depending on Windows display scaling).
pub fn tray_icon_size() -> u32 {
    unsafe { GetSystemMetrics(SM_CXSMICON).max(16) as u32 }
}

/// Render `text` centered in a `size`x`size` RGBA buffer (straight alpha,
/// row-major, top-down) using the native system font, shrink-to-fit so both
/// 2-3 digit values and a leading minus sign fit. `fg` colors the glyph;
/// `plate` optionally draws a background plate (reusing the Rounded/Badge
/// plate math from `sensors::tray`) beneath it. Returns an all-transparent
/// buffer if any GDI call fails — never panics. `bold` selects the tray
/// font weight (Bold vs. the default Medium — see `create_font`). `degree`
/// draws a small superscript `°` in a thin reserved column on the right
/// instead of appending it to `text` — appending it as a full glyph would
/// cost the shrink-to-fit loop ~30% of the digit size just to fit a 3rd
/// character (see `glyph_coverage_raw`).
pub fn render_number_rgba(
    text: &str,
    fg: (u8, u8, u8),
    plate: Option<((u8, u8, u8), TrayStyle)>,
    size: u32,
    bold: bool,
    degree: bool,
) -> Vec<u8> {
    let coverage = glyph_coverage(text, size, bold, degree);
    let mut out = vec![0u8; (size as usize) * (size as usize) * 4];
    for y in 0..size {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            let plate_px = match plate {
                Some(((r, g, b), TrayStyle::Rounded)) => {
                    plate_pixel(x, y, size, (r / 3, g / 3, b / 3), 190, 6)
                }
                Some(((r, g, b), TrayStyle::Badge)) => {
                    plate_pixel(x, y, size, (r, g, b), 235, size / 2)
                }
                _ => None,
            };
            let a = coverage[(y * size + x) as usize] as f32 / 255.0;
            let (fr, fgc, fb) = fg;
            let top = (fr as f32 / 255.0, fgc as f32 / 255.0, fb as f32 / 255.0, a);
            let bottom = match plate_px {
                Some([r, g, b, pa]) => (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, pa as f32 / 255.0),
                None => (0.0, 0.0, 0.0, 0.0),
            };
            let px = composite_over(top, bottom);
            out[i] = px.0;
            out[i + 1] = px.1;
            out[i + 2] = px.2;
            out[i + 3] = px.3;
        }
    }
    out
}

/// Porter-Duff "A over B" for straight (non-premultiplied) RGBA channels in
/// `0.0..=1.0`, returning straight 0-255 output.
fn composite_over(top: (f32, f32, f32, f32), bottom: (f32, f32, f32, f32)) -> (u8, u8, u8, u8) {
    let (tr, tg, tb, ta) = top;
    let (br, bg, bb, ba) = bottom;
    let oa = ta + ba * (1.0 - ta);
    if oa <= 0.0001 {
        return (0, 0, 0, 0);
    }
    let or_ = (tr * ta + br * ba * (1.0 - ta)) / oa;
    let og = (tg * ta + bg * ba * (1.0 - ta)) / oa;
    let ob = (tb * ta + bb * ba * (1.0 - ta)) / oa;
    (
        (or_ * 255.0).round().clamp(0.0, 255.0) as u8,
        (og * 255.0).round().clamp(0.0, 255.0) as u8,
        (ob * 255.0).round().clamp(0.0, 255.0) as u8,
        (oa * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// GDI hints and small-icon shrink-to-fit are coarse at the notification
/// area's native 16-20px size, which is what read as a "made up font" —
/// Segoe UI *is* the real typeface, just mangled by rasterizing straight at
/// that tiny size. Rendering at `SUPERSAMPLE`x and box-filtering down gives
/// GDI room to hint properly, producing a crisper, more faithful downscaled
/// glyph — the same trick used for supersampled font/icon rendering elsewhere.
const SUPERSAMPLE: u32 = 4;

/// Draws `text` at native size via supersampled GDI rasterization (see
/// `glyph_coverage_raw`) and box-filters the result down to `size`x`size`,
/// returning per-pixel alpha coverage.
fn glyph_coverage(text: &str, size: u32, bold: bool, degree: bool) -> Vec<u8> {
    let hi_size = size * SUPERSAMPLE;
    let hi = glyph_coverage_raw(text, hi_size, (6 * SUPERSAMPLE) as i32, bold, degree);
    downsample_box(&hi, hi_size, size)
}

/// Averages `src_size`x`src_size` coverage down to `dst_size`x`dst_size` in
/// uniform `src_size / dst_size` blocks (an integer factor by construction —
/// `src_size` is always `dst_size * SUPERSAMPLE`).
fn downsample_box(src: &[u8], src_size: u32, dst_size: u32) -> Vec<u8> {
    let factor = src_size / dst_size;
    let mut out = vec![0u8; (dst_size as usize) * (dst_size as usize)];
    for y in 0..dst_size {
        for x in 0..dst_size {
            let mut sum: u32 = 0;
            for sy in 0..factor {
                for sx in 0..factor {
                    let sp = (y * factor + sy) * src_size + (x * factor + sx);
                    sum += src[sp as usize] as u32;
                }
            }
            out[(y * dst_size + x) as usize] = (sum / (factor * factor)) as u8;
        }
    }
    out
}

/// Draws `text` white-on-black into an offscreen 32bpp top-down DIB with GDI
/// (grayscale-antialiased, NOT ClearType — ClearType's subpixel RGB fringing
/// would corrupt the coverage-as-alpha read below), shrinking the font until
/// it fits (down to `min_px_h`), then reads the DIB's R channel back as
/// per-pixel alpha coverage (GDI writes no real alpha; on a black background,
/// white-text antialiasing coverage IS the pixel's gray level, and R=G=B for
/// true gray). Returns an all-zero (fully transparent) buffer on any GDI
/// failure.
///
/// When `degree` is set, the digits are shrink-to-fit into a narrower column
/// (`size` minus a thin reserved strip on the right, `DEGREE_RESERVE_PCT` of
/// `size`) instead of the full width, and a `°` is drawn separately into that
/// reserved strip at roughly half the digit height, top-aligned with the
/// digits' top edge — like a superscript. This costs the digits only that
/// thin strip (a ~1px shrink at native tray size) instead of the ~30% they'd
/// lose if `°` were appended as a 3rd full-width glyph.
fn glyph_coverage_raw(text: &str, size: u32, min_px_h: i32, bold: bool, degree: bool) -> Vec<u8> {
    const DEGREE_RESERVE_PCT: u32 = 22;
    let mut coverage = vec![0u8; (size as usize) * (size as usize)];
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let digit_width = if degree {
        size.saturating_sub(size * DEGREE_RESERVE_PCT / 100)
    } else {
        size
    };
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return coverage;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of_val(&bmi.bmiHeader) as u32;
        bmi.bmiHeader.biWidth = size as i32;
        bmi.bmiHeader.biHeight = -(size as i32); // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) if !b.is_invalid() && !bits.is_null() => b,
            _ => {
                let _ = DeleteDC(dc);
                return coverage;
            }
        };
        let old_bitmap = SelectObject(dc, bitmap);

        let mut px_h = (size as i32 * 9 / 10).max(min_px_h);
        let mut font = create_font(px_h, bold);
        let old_font = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT);
        let _ = SetTextColor(dc, COLORREF(0x00FF_FFFF));

        loop {
            let mut extent = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &utf16, &mut extent);
            if extent.cx <= (digit_width as i32 - 1) || px_h <= min_px_h {
                break;
            }
            SelectObject(dc, old_font);
            let _ = DeleteObject(font);
            px_h -= 1;
            font = create_font(px_h, bold);
            SelectObject(dc, font);
        }

        let mut rect = RECT { left: 0, top: 0, right: digit_width as i32, bottom: size as i32 };
        let mut draw_buf = utf16.clone();
        DrawTextW(dc, &mut draw_buf, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        // Deselect and free the digit font before drawing the degree mark —
        // GDI disallows deleting a font while it's still selected into the DC.
        SelectObject(dc, old_font);
        let _ = DeleteObject(font);

        if degree {
            let deg_px_h = (px_h / 2).max((4 * SUPERSAMPLE) as i32);
            let deg_font = create_font(deg_px_h, bold);
            SelectObject(dc, deg_font);

            let digit_top = ((size as i32) - px_h) / 2;
            let mut deg_rect = RECT {
                left: digit_width as i32,
                top: digit_top,
                right: size as i32,
                bottom: digit_top + deg_px_h,
            };
            let mut deg_buf: Vec<u16> = "°".encode_utf16().collect();
            DrawTextW(dc, &mut deg_buf, &mut deg_rect, DT_CENTER | DT_SINGLELINE);

            SelectObject(dc, old_font);
            let _ = DeleteObject(deg_font);
        }

        let _ = GdiFlush();

        let px_count = (size as usize) * (size as usize);
        let bgra = std::slice::from_raw_parts(bits as *const u8, px_count * 4);
        for i in 0..px_count {
            coverage[i] = bgra[i * 4 + 2]; // BGRA -> R channel == AA coverage (gray)
        }

        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(dc);
    }
    coverage
}

fn create_font(px_height: i32, bold: bool) -> HFONT {
    let weight = if bold { FW_BOLD.0 } else { FW_MEDIUM.0 };
    unsafe {
        CreateFontW(
            -px_height,
            0,
            0,
            0,
            weight as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            ANTIALIASED_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 as u32) | (FF_DONTCARE.0 as u32),
            &HSTRING::from("Segoe UI"),
        )
    }
}
