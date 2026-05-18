//! Tonal palette: a hue+chroma pair sampled at 16 canonical M3 tones.
//!
//! The 16 stops are the union of the M3 canonical tone scale used across the
//! spec: 0, 10, 20, 25, 30, 35, 40, 50, 60, 70, 80, 90, 95, 98, 99, 100.

use crate::hct::Hct;
use serde::{Deserialize, Serialize};

/// M3 canonical tone stops (16 values).
pub const TONE_STOPS: [u8; 16] = [0, 10, 20, 25, 30, 35, 40, 50, 60, 70, 80, 90, 95, 98, 99, 100];

/// A tonal palette: a single (hue, chroma) anchor sampled at any tone.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TonalPalette {
    /// HCT hue in degrees [0, 360).
    pub hue: f64,
    /// HCT chroma (≥ 0).
    pub chroma: f64,
}

impl TonalPalette {
    /// Build a palette from a seed ARGB color. Hue + chroma are extracted via
    /// HCT; tone is discarded — the palette can be sampled at any tone.
    #[must_use]
    pub fn from_seed_argb(argb: u32) -> Self {
        let hct = Hct::from_argb(argb);
        Self {
            hue: hct.hue,
            chroma: hct.chroma,
        }
    }

    /// Build a palette with an explicit hue + chroma (used for fixed M3
    /// neutral/error palettes).
    #[must_use]
    pub fn from_hue_and_chroma(hue: f64, chroma: f64) -> Self {
        Self { hue, chroma }
    }

    /// Sample the palette at the requested tone (0..=100). Returns ARGB.
    ///
    /// `tone(0)` is clamped to pure black and `tone(100)` to pure white —
    /// matching `material-color-utilities`, which collapses gamut to grey at
    /// the L\* extremes (any chroma is unrepresentable there).
    #[must_use]
    pub fn tone(&self, t: u8) -> u32 {
        let t = t.min(100);
        if t == 0 {
            return 0xFF00_0000;
        }
        if t == 100 {
            return 0xFFFF_FFFF;
        }
        Hct {
            hue: self.hue,
            chroma: self.chroma,
            tone: f64::from(t),
        }
        .to_argb()
    }

    /// Return all 16 canonical M3 tones as `(tone, argb)` pairs.
    #[must_use]
    pub fn stops(&self) -> Vec<(u8, u32)> {
        TONE_STOPS.iter().map(|&t| (t, self.tone(t))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonal_palette_13_tones_monotonic() {
        // 13+ canonical M3 tones, monotonically non-decreasing luminance.
        let palette = TonalPalette::from_seed_argb(0xFF67_50A4);
        let stops = palette.stops();
        assert_eq!(stops.len(), 16);
        // Tone 0 → near black, tone 100 → near white.
        let (_, t0) = stops[0];
        let (_, t100) = stops[stops.len() - 1];
        let luma = |c: u32| ((c >> 16) & 0xFF) + ((c >> 8) & 0xFF) + (c & 0xFF);
        assert!(luma(t0) < 30, "tone 0 should be ~black (got {:06X})", t0);
        assert!(luma(t100) > 700, "tone 100 should be ~white (got {:06X})", t100);

        // Monotonicity over the full stop set.
        for w in stops.windows(2) {
            let (_, a) = w[0];
            let (_, b) = w[1];
            assert!(
                luma(a) <= luma(b) + 5,
                "non-monotonic: {:06X} > {:06X}",
                a,
                b
            );
        }
    }

    #[test]
    fn palette_tone_40_close_to_seed_for_default_purple() {
        // M3 default seed #6750A4 is itself at tone ~40, so tone(40) should
        // be close to the seed.
        let palette = TonalPalette::from_seed_argb(0xFF67_50A4);
        let t40 = palette.tone(40);
        let dr = (((t40 >> 16) & 0xFF) as i32 - 0x67).abs();
        let dg = (((t40 >> 8) & 0xFF) as i32 - 0x50).abs();
        let db = ((t40 & 0xFF) as i32 - 0xA4).abs();
        // Liberal bound: LCh chroma + tone re-solve can drift a bit.
        assert!(
            dr + dg + db < 60,
            "tone(40) {:06X} too far from seed 6750A4 (Δ={})",
            t40,
            dr + dg + db
        );
    }
}
