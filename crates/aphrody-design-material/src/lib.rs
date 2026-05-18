//! `aphrody-design-material` — Material Design 3 skin of open-design.
//!
//! Pure-Rust implementation of the canonical M3 system: HCT color space
//! (approximation via CIE Lab L\*ch), tonal palettes, 4 schemes
//! (light / dark / high-contrast light / high-contrast dark), state layers,
//! shape, elevation, motion, typography, and 6 visual directions including
//! the flagship "Material Expressive".

pub mod directions;
pub mod elevation;
pub mod hct;
pub mod motion;
pub mod registry;
pub mod scheme;
pub mod shape;
pub mod state_layers;
pub mod tonal_palette;
pub mod typography;

pub use directions::{Density, Direction};
pub use elevation::Elevation;
pub use hct::Hct;
pub use motion::{DurationToken, Easing, DURATIONS_MS};
pub use registry::{DesignSystemRegistry, Manifest, OwnedDirection, RegistryError};
pub use scheme::{CorePalettes, Scheme, SchemeVariant};
pub use shape::ShapeFamily;
pub use state_layers::{apply_state, composite_over, State, DRAGGED, FOCUS, HOVER, PRESSED};
pub use tonal_palette::{TonalPalette, TONE_STOPS};
pub use typography::{TypeRole, TypeStyle, FONT_BODY, FONT_DISPLAY};

/// Parse a `#RRGGBB` or `#AARRGGBB` hex string into an ARGB u32.
/// `#RRGGBB` is interpreted with alpha = 0xFF.
pub fn parse_hex_argb(s: &str) -> anyhow::Result<u32> {
    let s = s.trim().trim_start_matches('#');
    match s.len() {
        6 => {
            let v = u32::from_str_radix(s, 16)?;
            Ok(0xFF00_0000 | v)
        }
        8 => Ok(u32::from_str_radix(s, 16)?),
        n => Err(anyhow::anyhow!(
            "invalid hex length {n} (expected 6 or 8 nibbles)"
        )),
    }
}

/// Format an ARGB u32 as `#RRGGBB` (alpha dropped).
#[must_use]
pub fn format_hex_rgb(argb: u32) -> String {
    format!("#{:06X}", argb & 0x00FF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_roundtrip() {
        assert_eq!(parse_hex_argb("#6750A4").unwrap(), 0xFF67_50A4);
        assert_eq!(parse_hex_argb("6750A4").unwrap(), 0xFF67_50A4);
        assert_eq!(parse_hex_argb("#FF6750A4").unwrap(), 0xFF67_50A4);
        assert_eq!(format_hex_rgb(0xFF67_50A4), "#6750A4");
    }
}
