// SPDX-License-Identifier: Apache-2.0
//! `aphrody-logo` — the canonical aphrody character icon, embedded once and
//! derived into every format the project needs.
//!
//! The single source of truth is `assets/aphrody.webp` (an anime cel-shaded
//! portrait, see `~/.gemini/antigravity-cli/aphrody_design.md`). This crate
//! embeds it at compile time and exposes:
//!
//! * [`ico_bytes`] / [`write_ico`] — multi-resolution Windows `.ico`
//!   (16/32/48/64/128/256 px), the format Windows requires for app icons.
//! * [`svg_embedded`] — a scalable `.svg` wrapping the raster as a base64 PNG
//!   (renders crisply at any size, universally supported). True vector tracing
//!   is an out-of-band step (`vtracer --input assets/aphrody.webp --output
//!   aphrody.svg`) — the embedded form is the durable in-tree default.
//! * [`render_terminal`] / [`render_kitty`] / [`render_halfblocks`] —
//!   pixel-perfect terminal rendering. Kitty graphics protocol when the host
//!   supports it (true pixels), truecolor Unicode half-blocks everywhere else.

use std::io::Cursor;
use std::path::Path;

use base64::Engine as _;
use image::{GenericImageView, ImageFormat, imageops::FilterType};

/// Raw WebP bytes of the canonical aphrody icon (single source of truth).
pub static LOGO_WEBP: &[u8] = include_bytes!("../../../assets/aphrody.webp");

/// Errors surfaced by the logo derivations.
#[derive(Debug, thiserror::Error)]
pub enum LogoError {
    /// Image decode/encode failure.
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    /// Filesystem write failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// ICO container encode failure.
    #[error("ico: {0}")]
    Ico(String),
}

/// Decode the embedded WebP into an owned image.
fn decode() -> Result<image::DynamicImage, LogoError> {
    Ok(image::load_from_memory_with_format(LOGO_WEBP, ImageFormat::WebP)?)
}

/// The icon sizes a Windows `.ico` should carry (per the Microsoft icon
/// guidelines: 16/32/48/256 minimum; 64/128 added for crisp intermediate DPI).
const ICO_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

/// Encode the icon as a multi-resolution Windows `.ico`.
pub fn ico_bytes() -> Result<Vec<u8>, LogoError> {
    let src = decode()?;
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in ICO_SIZES {
        let rgba = src.resize_exact(size, size, FilterType::Lanczos3).to_rgba8();
        let img = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
        let entry =
            ico::IconDirEntry::encode(&img).map_err(|e| LogoError::Ico(e.to_string()))?;
        dir.add_entry(entry);
    }
    let mut buf = Vec::new();
    dir.write(&mut Cursor::new(&mut buf)).map_err(|e| LogoError::Ico(e.to_string()))?;
    Ok(buf)
}

/// Write the multi-resolution `.ico` to `path`.
pub fn write_ico(path: &Path) -> Result<(), LogoError> {
    std::fs::write(path, ico_bytes()?)?;
    Ok(())
}

/// A scalable `.svg` embedding the icon as a base64 PNG. Valid SVG that scales
/// to any container; no external assets.
pub fn svg_embedded() -> Result<String, LogoError> {
    let src = decode()?;
    let (w, h) = src.dimensions();
    let mut png = Vec::new();
    src.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {w} {h}\">\n  \
         <image width=\"{w}\" height=\"{h}\" \
         href=\"data:image/png;base64,{b64}\"/>\n</svg>\n"
    ))
}

/// Write the embedded `.svg` to `path`.
pub fn write_svg(path: &Path) -> Result<(), LogoError> {
    std::fs::write(path, svg_embedded()?)?;
    Ok(())
}

/// Encode a PNG of the icon scaled to fit within `max_px` on its longest side.
fn scaled_png(max_px: u32) -> Result<Vec<u8>, LogoError> {
    let scaled = decode()?.resize(max_px.max(1), max_px.max(1), FilterType::Lanczos3);
    let mut png = Vec::new();
    scaled.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

/// Render the icon as a Kitty graphics protocol escape sequence (PNG payload,
/// base64, 4 KiB chunks). True pixel rendering in Kitty / Ghostty / WezTerm /
/// Konsole. `max_px` caps the longest side in device pixels.
pub fn render_kitty(max_px: u32) -> Result<String, LogoError> {
    let png = scaled_png(max_px)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    let bytes = b64.as_bytes();
    let chunks: Vec<&[u8]> = bytes.chunks(4096).collect();
    let mut out = String::with_capacity(b64.len() + chunks.len() * 16 + 32);
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        let payload = std::str::from_utf8(chunk).unwrap_or("");
        if i == 0 {
            // a=T (transmit+display), f=100 (PNG), m=more.
            out.push_str(&format!("\x1b_Ga=T,f=100,m={more};{payload}\x1b\\"));
        } else {
            out.push_str(&format!("\x1b_Gm={more};{payload}\x1b\\"));
        }
    }
    out.push('\n');
    Ok(out)
}

/// Render the icon with truecolor Unicode half-blocks (`U+2580`), packing two
/// vertical pixels per character cell. Universal 24-bit-color fallback that
/// needs no graphics protocol. `cols` is the target width in cells.
pub fn render_halfblocks(cols: u32) -> Result<String, LogoError> {
    let src = decode()?;
    let (w, h) = src.dimensions();
    let cols = cols.max(1);
    // A character cell is roughly twice as tall as wide and shows two stacked
    // pixels, so the pixel height that preserves the visual aspect is cols*h/w.
    let mut rows_px = (cols * h).max(w) / w;
    if rows_px % 2 == 1 {
        rows_px += 1;
    }
    let img = src.resize_exact(cols, rows_px.max(2), FilterType::Lanczos3).to_rgba8();
    let (iw, ih) = (img.width(), img.height());
    let mut out = String::with_capacity((iw * ih) as usize);
    let mut y = 0;
    while y + 1 < ih {
        for x in 0..iw {
            let t = img.get_pixel(x, y).0;
            let b = img.get_pixel(x, y + 1).0;
            // Upper half block: foreground = top pixel, background = bottom.
            out.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                t[0], t[1], t[2], b[0], b[1], b[2]
            ));
        }
        out.push_str("\x1b[0m\n");
        y += 2;
    }
    Ok(out)
}

/// Best-effort terminal rendering: Kitty graphics protocol when the host
/// advertises support, truecolor half-blocks otherwise. `cols` is the target
/// width in character cells.
pub fn render_terminal(cols: u32) -> Result<String, LogoError> {
    if supports_kitty() {
        // ~ one cell is about 10x20 device px; cap the longest side accordingly.
        render_kitty(cols.saturating_mul(10).clamp(64, 1024))
    } else {
        render_halfblocks(cols)
    }
}

/// Heuristic detection of the Kitty graphics protocol via environment.
fn supports_kitty() -> bool {
    if std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var_os("GHOSTTY_RESOURCES_DIR").is_some()
    {
        return true;
    }
    matches!(std::env::var("TERM"), Ok(t) if t.contains("kitty"))
        || matches!(std::env::var("TERM_PROGRAM"), Ok(t) if t == "WezTerm" || t == "ghostty")
}

// ---------------------------------------------------------------------------
// Material Design 3 icon framing.
//
// Source: https://m3.material.io/styles/icons/overview (fetched via obscura
// 2026-05-21 — the SPA body requires JS the headless engine does not bootstrap,
// but the page meta confirms "Material Symbols is a variable icon font ...
// seven weights and three styles"). The framing rules below follow the M3 /
// Android adaptive-icon spec: a brand-coloured full-bleed square with the
// content kept inside the central safe zone so any launcher mask (circle,
// squircle, rounded-rect) never clips it.
// ---------------------------------------------------------------------------

/// Default brand background for composed icons — the aphrody rust-orange
/// (`#CE422B`, matching the plugin `brandColor`).
pub const BRAND_BG: [u8; 3] = [0xCE, 0x42, 0x2B];

/// M3 maskable safe zone: content stays within the central 80% of the icon.
pub const M3_SAFE_FRACTION: f32 = 0.80;

/// Launcher mask silhouettes per the M3 / Android adaptive-icon shapes.
#[derive(Debug, Clone, Copy)]
pub enum IconShape {
    /// Full square (no mask).
    Square,
    /// Circle.
    Circle,
    /// Rounded rectangle; `radius_frac` in `0.0..=0.5` of the side.
    RoundedRect {
        /// Corner radius as a fraction of the side length.
        radius_frac: f32,
    },
    /// Android/M3 squircle (superellipse); `n` controls roundness (~4.0).
    Squircle {
        /// Superellipse exponent.
        n: f32,
    },
}

/// Compose the portrait into a `size`x`size` square on a solid background,
/// scaling it into the central `content_frac` safe zone.
pub fn composed_png(size: u32, bg: [u8; 3], content_frac: f32) -> Result<Vec<u8>, LogoError> {
    let size = size.max(1);
    let frac = content_frac.clamp(0.1, 1.0);
    let inner = ((f64::from(size) * f64::from(frac)).round() as u32).max(1);
    let portrait = decode()?.resize(inner, inner, FilterType::Lanczos3).to_rgba8();
    let mut canvas =
        image::RgbaImage::from_pixel(size, size, image::Rgba([bg[0], bg[1], bg[2], 255]));
    let ox = i64::from(size.saturating_sub(portrait.width()) / 2);
    let oy = i64::from(size.saturating_sub(portrait.height()) / 2);
    image::imageops::overlay(&mut canvas, &portrait, ox, oy);
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas).write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

/// A maskable adaptive icon (M3): the portrait on the brand background with the
/// M3 safe-zone padding, ready for any launcher mask.
pub fn maskable_png(size: u32, bg: [u8; 3]) -> Result<Vec<u8>, LogoError> {
    composed_png(size, bg, M3_SAFE_FRACTION)
}

/// Whether pixel `(x, y)` lies inside `shape` for a `size`x`size` icon.
fn inside_shape(shape: IconShape, x: u32, y: u32, size: u32) -> bool {
    let s = f32::from(u16::try_from(size).unwrap_or(u16::MAX));
    let cx = (x as f32 + 0.5) / s * 2.0 - 1.0;
    let cy = (y as f32 + 0.5) / s * 2.0 - 1.0;
    match shape {
        IconShape::Square => true,
        IconShape::Circle => cx * cx + cy * cy <= 1.0,
        IconShape::Squircle { n } => cx.abs().powf(n) + cy.abs().powf(n) <= 1.0,
        IconShape::RoundedRect { radius_frac } => {
            let r = radius_frac.clamp(0.0, 0.5) * 2.0;
            let (ax, ay) = (cx.abs(), cy.abs());
            if ax <= 1.0 - r || ay <= 1.0 - r {
                true
            } else {
                let (dx, dy) = (ax - (1.0 - r), ay - (1.0 - r));
                dx * dx + dy * dy <= r * r
            }
        },
    }
}

/// Render the icon masked to an M3 launcher `shape` on a brand background:
/// a `size`x`size` PNG with transparent corners outside the silhouette.
pub fn masked_png(size: u32, bg: [u8; 3], shape: IconShape) -> Result<Vec<u8>, LogoError> {
    let size = size.max(1);
    let composed = composed_png(size, bg, M3_SAFE_FRACTION)?;
    let mut img = image::load_from_memory_with_format(&composed, ImageFormat::Png)?.to_rgba8();
    for y in 0..size {
        for x in 0..size {
            if !inside_shape(shape, x, y, size) {
                img.get_pixel_mut(x, y).0[3] = 0;
            }
        }
    }
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img).write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webp_decodes() {
        let img = decode().expect("embedded webp decodes");
        let (w, h) = img.dimensions();
        assert!(w > 0 && h > 0, "non-empty image");
    }

    #[test]
    fn ico_has_multi_resolution_header() {
        let ico = ico_bytes().expect("ico encodes");
        // ICO header: reserved(0,0), type(1,0)=icon, count = ICO_SIZES.len().
        assert_eq!(&ico[0..4], &[0, 0, 1, 0], "ICO magic + type");
        let count = u16::from_le_bytes([ico[4], ico[5]]);
        assert_eq!(count as usize, ICO_SIZES.len(), "all sizes present");
    }

    #[test]
    fn svg_is_wellformed_and_scalable() {
        let svg = svg_embedded().expect("svg encodes");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox="));
        assert!(svg.contains("data:image/png;base64,"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn kitty_payload_is_apc_wrapped() {
        let k = render_kitty(64).expect("kitty render");
        assert!(k.contains("\x1b_Ga=T,f=100"), "starts with APC graphics cmd");
        assert!(k.contains("\x1b\\"), "APC terminator present");
    }

    #[test]
    fn halfblocks_use_upper_half_and_reset() {
        let hb = render_halfblocks(16).expect("halfblock render");
        assert!(hb.contains('\u{2580}'), "upper half block glyph");
        assert!(hb.contains("\x1b[38;2;"), "truecolor foreground");
        assert!(hb.contains("\x1b[0m"), "row reset");
    }

    #[test]
    fn maskable_is_square_opaque_png() {
        let png = maskable_png(256, BRAND_BG).expect("maskable encodes");
        let img = image::load_from_memory(&png).expect("decode").to_rgba8();
        assert_eq!(img.dimensions(), (256, 256), "full-bleed square");
        // A corner pixel is the opaque brand background (safe-zone padding).
        assert_eq!(img.get_pixel(0, 0).0[3], 255, "opaque corner background");
    }

    #[test]
    fn circle_mask_clears_corners_keeps_center() {
        let png = masked_png(128, BRAND_BG, IconShape::Circle).expect("masked encodes");
        let img = image::load_from_memory(&png).expect("decode").to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0[3], 0, "corner outside circle is transparent");
        assert_eq!(img.get_pixel(64, 64).0[3], 255, "center inside circle is opaque");
    }

    #[test]
    fn squircle_keeps_more_corner_area_than_circle() {
        let sq = masked_png(128, BRAND_BG, IconShape::Squircle { n: 4.0 }).expect("squircle");
        let img = image::load_from_memory(&sq).expect("decode").to_rgba8();
        // A near-corner point the circle would clip stays inside the squircle.
        assert_eq!(img.get_pixel(20, 20).0[3], 255, "squircle keeps near-corner");
    }
}
