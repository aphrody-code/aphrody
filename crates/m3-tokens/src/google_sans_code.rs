// SPDX-License-Identifier: Apache-2.0
//! Google Sans Code variable-font axis tokens.

/// Raw TTF bytes for Google Sans Code.
/// (To be downloaded into assets/fonts/google-sans-code/GoogleSansCode-Variable.ttf)
#[cfg(feature = "embedded-fonts")]
pub const TTF_DATA: &[u8] = include_bytes!("../../../assets/fonts/google-sans-code/GoogleSansCode-Variable.ttf");

/// Google Sans Code variable-font axis tokens.
#[derive(Debug, Clone, Copy)]
pub struct GoogleSansCodeAxes {
    /// Monospace axis (0.0 to 1.0).
    pub mono: f32,
    /// Font weight axis (300.0 to 800.0).
    pub wght: f32,
}

impl Default for GoogleSansCodeAxes {
    fn default() -> Self {
        Self { mono: 1.0, wght: 400.0 }
    }
}
