// SPDX-License-Identifier: Apache-2.0
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

extern crate alloc;

use alloc::string::String;
use alloc::format;

/// Raw WOFF2 bytes for the `Material Symbols Rounded` variable font.
pub const WOFF2_ROUNDED: &[u8] = include_bytes!("../material-symbols/material-symbols-rounded.woff2");

/// Raw WOFF2 bytes for the `Material Symbols Outlined` variable font.
pub const WOFF2_OUTLINED: &[u8] = include_bytes!("../material-symbols/material-symbols-outlined.woff2");

/// Raw WOFF2 bytes for the `Material Symbols Sharp` variable font.
pub const WOFF2_SHARP: &[u8] = include_bytes!("../material-symbols/material-symbols-sharp.woff2");

/// Raw bytes for `Material Icons Regular` font.
pub const WOFF2_ICONS_REGULAR: &[u8] = include_bytes!("../material-icons/material-icons-regular.woff2");

/// Raw bytes for `Material Icons Outlined` font.
pub const WOFF2_ICONS_OUTLINED: &[u8] = include_bytes!("../material-icons/material-icons-outlined.woff2");

/// Raw bytes for `Material Icons Round` font.
pub const WOFF2_ICONS_ROUND: &[u8] = include_bytes!("../material-icons/material-icons-round.woff2");

/// Raw bytes for `Material Icons Sharp` font.
pub const WOFF2_ICONS_SHARP: &[u8] = include_bytes!("../material-icons/material-icons-sharp.woff2");

/// Raw bytes for `Material Icons Two Tone` font.
pub const WOFF2_ICONS_TWO_TONE: &[u8] = include_bytes!("../material-icons/material-icons-two-tone.woff2");

pub const CSS_ICONS_REGULAR: &str = include_str!("../material-icons/regular.css");
pub const CSS_ICONS_OUTLINED: &str = include_str!("../material-icons/outlined.css");
pub const CSS_ICONS_ROUND: &str = include_str!("../material-icons/round.css");
pub const CSS_ICONS_SHARP: &str = include_str!("../material-icons/sharp.css");
pub const CSS_ICONS_TWO_TONE: &str = include_str!("../material-icons/two-tone.css");

/// CSS definitions for `Material Symbols Rounded`.
pub const CSS_ROUNDED: &str = include_str!("../material-symbols/rounded.css");

/// CSS definitions for `Material Symbols Outlined`.
pub const CSS_OUTLINED: &str = include_str!("../material-symbols/outlined.css");

/// CSS definitions for `Material Symbols Sharp`.
pub const CSS_SHARP: &str = include_str!("../material-symbols/sharp.css");

/// A helper enum representing the available Material Symbol and Icon styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SymbolStyle {
    Rounded,
    Outlined,
    Sharp,
    IconsRegular,
    IconsOutlined,
    IconsRound,
    IconsSharp,
    IconsTwoTone,
}

impl SymbolStyle {
    /// Returns the raw WOFF2 font data for the given style.
    pub const fn woff2_data(&self) -> &'static [u8] {
        match self {
            Self::Rounded => WOFF2_ROUNDED,
            Self::Outlined => WOFF2_OUTLINED,
            Self::Sharp => WOFF2_SHARP,
            Self::IconsRegular => WOFF2_ICONS_REGULAR,
            Self::IconsOutlined => WOFF2_ICONS_OUTLINED,
            Self::IconsRound => WOFF2_ICONS_ROUND,
            Self::IconsSharp => WOFF2_ICONS_SHARP,
            Self::IconsTwoTone => WOFF2_ICONS_TWO_TONE,
        }
    }

    /// Returns the CSS snippet required to use this font style on the web.
    pub const fn css(&self) -> &'static str {
        match self {
            Self::Rounded => CSS_ROUNDED,
            Self::Outlined => CSS_OUTLINED,
            Self::Sharp => CSS_SHARP,
            Self::IconsRegular => CSS_ICONS_REGULAR,
            Self::IconsOutlined => CSS_ICONS_OUTLINED,
            Self::IconsRound => CSS_ICONS_ROUND,
            Self::IconsSharp => CSS_ICONS_SHARP,
            Self::IconsTwoTone => CSS_ICONS_TWO_TONE,
        }
    }

    /// Returns the CSS `font-family` name for this style.
    pub const fn font_family(&self) -> &'static str {
        match self {
            Self::Rounded => "Material Symbols Rounded",
            Self::Outlined => "Material Symbols Outlined",
            Self::Sharp => "Material Symbols Sharp",
            Self::IconsRegular => "Material Icons",
            Self::IconsOutlined => "Material Icons Outlined",
            Self::IconsRound => "Material Icons Round",
            Self::IconsSharp => "Material Icons Sharp",
            Self::IconsTwoTone => "Material Icons Two Tone",
        }
    }

    /// Returns the CSS snippet with the WOFF2 font natively embedded as base64.
    pub fn base64_css(&self) -> String {
        use base64::prelude::*;
        let data = self.woff2_data();
        let encoded = BASE64_STANDARD.encode(data);
        let family = self.font_family();
        format!(
            "@font-face {{
              font-family: \"{family}\";
              font-style: normal;
              font-weight: 100 700;
              font-display: block;
              src: url(\"data:font/woff2;charset=utf-8;base64,{encoded}\") format(\"woff2\");
            }}
            .{class_name} {{
              font-family: \"{family}\";
              font-weight: normal;
              font-style: normal;
              font-size: 24px;
              line-height: 1;
              letter-spacing: normal;
              text-transform: none;
              display: inline-block;
              white-space: nowrap;
              word-wrap: normal;
              direction: ltr;
              -webkit-font-smoothing: antialiased;
              -moz-osx-font-smoothing: grayscale;
              text-rendering: optimizeLegibility;
              font-feature-settings: \"liga\";
            }}",
            family = family,
            encoded = encoded,
            class_name = family.to_lowercase().replace(' ', "-"),
        )
    }
}
