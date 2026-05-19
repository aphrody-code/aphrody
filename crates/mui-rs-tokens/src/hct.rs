//! HCT color space — Hue/Chroma/Tone (Google Material You).
//! Custom Rust impl ported from material-color-utilities TypeScript reference.
//! See: docs/m3-llms/color-utils/typescript/hct/hct.ts

/// HCT color: perceptually uniform, used for M3 dynamic color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hct {
    pub hue: f64,
    pub chroma: f64,
    pub tone: f64,
}

impl Hct {
    pub fn new(hue: f64, chroma: f64, tone: f64) -> Self {
        Self { hue: hue.rem_euclid(360.0), chroma: chroma.max(0.0), tone: tone.clamp(0.0, 100.0) }
    }

    pub fn from_argb(argb: u32) -> Self {
        // TODO: implement CAM16 → HCT conversion (~600 LOC)
        let _ = argb;
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn to_argb(&self) -> u32 {
        // TODO: implement HCT → CAM16 → sRGB
        0xFF000000
    }
}
