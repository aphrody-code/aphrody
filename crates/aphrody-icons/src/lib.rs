#![no_std]
#![forbid(unsafe_code)]

//! `aphrody-icons` — Material Design Icons (Material Symbols) natively embedded for Rust.
//!
//! This crate embeds the official Google Material Symbols (`Rounded`, `Outlined`, `Sharp`)
//! as raw `WOFF2` bytes directly into the Rust binary. This allows native GPU renderers
//! (`wgpu`, `vello`, `parley`) and WASM applications to load the fonts instantaneously
//! without filesystem IO or external HTTP requests.
//!
//! It also exports the associated CSS for web-based GUI targets.

/// Raw WOFF2 bytes for the `Material Symbols Rounded` variable font.
pub const WOFF2_ROUNDED: &[u8] = include_bytes!("../material-symbols/material-symbols-rounded.woff2");

/// Raw WOFF2 bytes for the `Material Symbols Outlined` variable font.
pub const WOFF2_OUTLINED: &[u8] = include_bytes!("../material-symbols/material-symbols-outlined.woff2");

/// Raw WOFF2 bytes for the `Material Symbols Sharp` variable font.
pub const WOFF2_SHARP: &[u8] = include_bytes!("../material-symbols/material-symbols-sharp.woff2");

/// CSS definitions for `Material Symbols Rounded`.
pub const CSS_ROUNDED: &str = include_str!("../material-symbols/rounded.css");

/// CSS definitions for `Material Symbols Outlined`.
pub const CSS_OUTLINED: &str = include_str!("../material-symbols/outlined.css");

/// CSS definitions for `Material Symbols Sharp`.
pub const CSS_SHARP: &str = include_str!("../material-symbols/sharp.css");

/// A helper enum representing the three available Material Symbol styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SymbolStyle {
    Rounded,
    Outlined,
    Sharp,
}

impl SymbolStyle {
    /// Returns the raw WOFF2 font data for the given style.
    pub const fn woff2_data(&self) -> &'static [u8] {
        match self {
            Self::Rounded => WOFF2_ROUNDED,
            Self::Outlined => WOFF2_OUTLINED,
            Self::Sharp => WOFF2_SHARP,
        }
    }

    /// Returns the CSS snippet required to use this font style on the web.
    pub const fn css(&self) -> &'static str {
        match self {
            Self::Rounded => CSS_ROUNDED,
            Self::Outlined => CSS_OUTLINED,
            Self::Sharp => CSS_SHARP,
        }
    }

    /// Returns the CSS `font-family` name for this style.
    pub const fn font_family(&self) -> &'static str {
        match self {
            Self::Rounded => "Material Symbols Rounded",
            Self::Outlined => "Material Symbols Outlined",
            Self::Sharp => "Material Symbols Sharp",
        }
    }
}
