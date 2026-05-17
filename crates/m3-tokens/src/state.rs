// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 state-layer tokens.
//!
//! M3 state layers are semi-transparent overlays applied to interactive
//! components to communicate user interaction.  The overlay colour is derived
//! from the component's content colour (e.g. `on-primary`); only the opacity
//! changes between states.
//!
//! The six canonical states and their opacity values are:
//!
//! | Token                | Opacity | Use-case                              |
//! |----------------------|---------|---------------------------------------|
//! | [`HOVER`]            | 0.08    | Pointer resting on the component      |
//! | [`FOCUS`]            | 0.10    | Keyboard / programmatic focus         |
//! | [`PRESSED`]          | 0.10    | Touch or mouse button down            |
//! | [`DRAGGED`]          | 0.16    | Component being dragged               |
//! | [`DISABLED_CONTENT`] | 0.38    | Foreground (text/icon) when disabled  |
//! | [`DISABLED_CONTAINER`] | 0.12  | Container surface when disabled       |
//!
//! The `disabled` tokens differ in nature from the interaction tokens: they
//! reduce the opacity of the content and container colours rather than
//! compositing an overlay, but they are specified in the same M3 state table
//! and are therefore grouped here.
//!
//! Reference: <https://m3.material.io/foundations/interaction/states/overview>

/// Number of state tokens exposed by this module.
pub const STATE_COUNT: usize = 6;

/// A single M3 state-layer token carrying an opacity multiplier in `[0, 1]`.
///
/// The `opacity` value is `f32` so it can be fed directly to CSS custom
/// properties, WGSL shader uniforms, and platform colour APIs without
/// additional conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateLayer {
    /// Opacity of the state overlay, in the range `[0.0, 1.0]`.
    pub opacity: f32,
}

// ── Canonical M3 state-layer constants ───────────────────────────────────────

/// State layer applied while a pointer hovers over the component.
///
/// Opacity: `0.08`.
pub const HOVER: StateLayer = StateLayer { opacity: 0.08 };

/// State layer applied while the component holds keyboard or programmatic focus.
///
/// Opacity: `0.10`.
pub const FOCUS: StateLayer = StateLayer { opacity: 0.10 };

/// State layer applied while the component is being pressed.
///
/// Opacity: `0.10`.
pub const PRESSED: StateLayer = StateLayer { opacity: 0.10 };

/// State layer applied while the component is being dragged.
///
/// Opacity: `0.16`.
pub const DRAGGED: StateLayer = StateLayer { opacity: 0.16 };

/// Opacity applied to foreground (text / icon) content when a component is
/// disabled.
///
/// Opacity: `0.38`.
pub const DISABLED_CONTENT: StateLayer = StateLayer { opacity: 0.38 };

/// Opacity applied to the container surface when a component is disabled.
///
/// Opacity: `0.12`.
pub const DISABLED_CONTAINER: StateLayer = StateLayer { opacity: 0.12 };

// ── Aggregate arrays ─────────────────────────────────────────────────────────

/// All six state tokens in declaration order:
/// [`HOVER`], [`FOCUS`], [`PRESSED`], [`DRAGGED`],
/// [`DISABLED_CONTENT`], [`DISABLED_CONTAINER`].
pub const ALL: [StateLayer; STATE_COUNT] =
    [HOVER, FOCUS, PRESSED, DRAGGED, DISABLED_CONTENT, DISABLED_CONTAINER];

/// Human-readable names for each entry in [`ALL`], in the same order.
///
/// These strings mirror the M3 CSS custom-property suffixes used by
/// [`export_css`].
pub const NAMES: [&str; STATE_COUNT] =
    ["hover", "focus", "pressed", "dragged", "disabled-content", "disabled-container"];

// ── CSS export (std only) ─────────────────────────────────────────────────────

/// Emits a CSS `:root { … }` block declaring all M3 state-layer opacity
/// custom properties.
///
/// # Example output
///
/// ```css
/// :root {
///   --md-sys-state-hover-state-layer-opacity: 0.08;
///   --md-sys-state-focus-state-layer-opacity: 0.10;
///   --md-sys-state-pressed-state-layer-opacity: 0.10;
///   --md-sys-state-dragged-state-layer-opacity: 0.16;
///   --md-sys-state-disabled-content-state-layer-opacity: 0.38;
///   --md-sys-state-disabled-container-state-layer-opacity: 0.12;
/// }
/// ```
#[cfg(feature = "std")]
#[must_use]
pub fn export_css() -> String {
    let mut out = String::from(":root {\n");
    for (name, layer) in NAMES.iter().zip(ALL.iter()) {
        // Format with enough decimal places to round-trip the f32 faithfully
        // while keeping the output tidy (max 2 significant decimal digits
        // covers the full M3 opacity table).
        out.push_str(&std::format!(
            "  --md-sys-state-{name}-state-layer-opacity: {:.2};\n",
            layer.opacity
        ));
    }
    out.push('}');
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_len_is_six() {
        assert_eq!(ALL.len(), 6);
    }

    #[test]
    fn names_len_is_six() {
        assert_eq!(NAMES.len(), 6);
    }

    #[test]
    fn every_opacity_in_unit_range() {
        for layer in &ALL {
            assert!(
                (0.0..=1.0).contains(&layer.opacity),
                "opacity {} is outside [0.0, 1.0]",
                layer.opacity
            );
        }
    }

    #[test]
    fn hover_opacity_less_than_dragged() {
        assert!(
            HOVER.opacity < DRAGGED.opacity,
            "HOVER ({}) must be less than DRAGGED ({})",
            HOVER.opacity,
            DRAGGED.opacity
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_css_contains_hover_property() {
        let css = export_css();
        assert!(
            css.contains("--md-sys-state-hover"),
            "export_css must emit --md-sys-state-hover; got:\n{css}"
        );
    }
}
