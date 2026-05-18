// SPDX-License-Identifier: Apache-2.0
//
// Minimal Material 3 token bridge — direct port of `src/m3-tokens.ts`.
// The live dynamic palette is computed terminal-side by `crates/m3-tokens`;
// here we keep a tiny static fallback so this crate can resolve color names
// offline for tests, error messages, and the rare case where the renderer
// needs a hex value upfront.

//! Material 3 baseline token table (seed `#1A73E8`, TonalSpot dark scheme).

/// The 29 baseline M3 color-role token names recognized by aphrody-jsx.
pub const TOKEN_NAMES: &[&str] = &[
    "primary",
    "onPrimary",
    "primaryContainer",
    "onPrimaryContainer",
    "secondary",
    "onSecondary",
    "secondaryContainer",
    "onSecondaryContainer",
    "tertiary",
    "onTertiary",
    "tertiaryContainer",
    "onTertiaryContainer",
    "error",
    "onError",
    "errorContainer",
    "onErrorContainer",
    "background",
    "onBackground",
    "surface",
    "onSurface",
    "surfaceVariant",
    "onSurfaceVariant",
    "outline",
    "outlineVariant",
    "inverseSurface",
    "inverseOnSurface",
    "inversePrimary",
    "shadow",
    "scrim",
];

/// Static M3 dark-palette fallback table — exact byte-for-byte parity with
/// the TS source's `FALLBACK_DARK` constant.
pub const FALLBACK_DARK: &[(&str, &str)] = &[
    ("primary", "#A8C8FA"),
    ("onPrimary", "#003063"),
    ("primaryContainer", "#00468C"),
    ("onPrimaryContainer", "#D6E3FF"),
    ("secondary", "#BCC7DC"),
    ("onSecondary", "#263141"),
    ("secondaryContainer", "#3C4858"),
    ("onSecondaryContainer", "#D8E3F8"),
    ("tertiary", "#DCBCDF"),
    ("onTertiary", "#3F2844"),
    ("tertiaryContainer", "#573E5C"),
    ("onTertiaryContainer", "#F9D8FB"),
    ("error", "#FFB4AB"),
    ("onError", "#690005"),
    ("errorContainer", "#93000A"),
    ("onErrorContainer", "#FFDAD6"),
    ("background", "#101418"),
    ("onBackground", "#E1E2E8"),
    ("surface", "#101418"),
    ("onSurface", "#E1E2E8"),
    ("surfaceVariant", "#43474E"),
    ("onSurfaceVariant", "#C3C7CF"),
    ("outline", "#8D9199"),
    ("outlineVariant", "#43474E"),
    ("inverseSurface", "#E1E2E8"),
    ("inverseOnSurface", "#2E3035"),
    ("inversePrimary", "#0061A4"),
    ("shadow", "#000000"),
    ("scrim", "#000000"),
];

/// Returns `true` if `value` is the name of a known M3 token.
#[must_use]
pub fn is_m3_token(value: &str) -> bool {
    TOKEN_NAMES.iter().any(|name| *name == value)
}

/// Resolves a color string to a CSS-compatible hex value: tokens go through
/// the fallback table; arbitrary strings (raw hex / rgb / css names) pass
/// through unchanged.
#[must_use]
pub fn resolve_color(value: &str) -> &str {
    for (name, hex) in FALLBACK_DARK {
        if *name == value {
            return hex;
        }
    }
    value
}

/// Returns the marker the renderer should embed when forwarding a color token
/// to the terminal. Tokens stay symbolic (prefixed `m3:`) so the renderer can
/// re-resolve them against the live palette.
#[must_use]
pub fn encode_color(value: &str) -> alloc::string::String {
    use alloc::string::ToString;
    if is_m3_token(value) {
        alloc::format!("m3:{value}")
    } else {
        value.to_string()
    }
}
