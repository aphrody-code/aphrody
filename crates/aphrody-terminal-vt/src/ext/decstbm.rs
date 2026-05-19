// SPDX-License-Identifier: Apache-2.0
//! DECSTBM — set top/bottom scroll margins (`CSI Pt ; Pb r`).
//!
//! The terminal restricts vertical scrolling to the inclusive 1-based
//! line range `[Pt, Pb]`. If either parameter is missing it defaults to
//! the natural extreme (`Pt=1`, `Pb=rows`). The Microsoft Terminal
//! parser (`src/terminal/parser/InputStateMachineEngine.cpp`) rejects
//! invalid regions (top ≥ bottom) by falling back to the full screen,
//! and we mirror that behaviour to match what real-world programs
//! (Vim, less, htop) expect.

/// Inclusive 1-based scroll region (`top..=bottom`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollMargins {
    /// 1-based first scrollable row.
    pub top: u16,
    /// 1-based last scrollable row.
    pub bottom: u16,
}

/// Parse `CSI Pt ; Pb r` parameters into a [`ScrollMargins`].
///
/// `rows` is the terminal height (so `Pb` can default to the bottom
/// row, and so we can clamp out-of-range margins). The standard
/// defaults are `Pt=1` and `Pb=rows` when either is missing or zero.
///
/// **Validation** (matches Microsoft Terminal semantics): when the
/// resulting margins would be invalid (`top >= bottom`, or either
/// outside the grid), we fall back to the full-screen region
/// `(1, rows)`. This prevents a misbehaving sender from wedging the
/// scroll engine in an unusable state.
#[must_use]
pub fn parse_csi_r(params: &[u16], rows: u16) -> ScrollMargins {
    let rows = rows.max(1);
    let full = ScrollMargins { top: 1, bottom: rows };

    let raw_top = params.first().copied().unwrap_or(0);
    let raw_bot = params.get(1).copied().unwrap_or(0);

    // 0 (or missing) means "use the natural default".
    let top = if raw_top == 0 { 1 } else { raw_top };
    let bottom = if raw_bot == 0 { rows } else { raw_bot };

    // Clamp out-of-grid values to the grid, then reject invalid regions
    // by falling back to the full screen (matches Microsoft's parser).
    if top < 1 || bottom > rows || top >= bottom {
        return full;
    }
    ScrollMargins { top, bottom }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_top_bottom_honoured() {
        assert_eq!(parse_csi_r(&[5, 20], 30), ScrollMargins { top: 5, bottom: 20 });
    }

    #[test]
    fn defaults_to_full_screen() {
        assert_eq!(parse_csi_r(&[], 30), ScrollMargins { top: 1, bottom: 30 });
        assert_eq!(parse_csi_r(&[0, 0], 30), ScrollMargins { top: 1, bottom: 30 });
    }

    #[test]
    fn invalid_top_ge_bottom_falls_back() {
        // Microsoft Terminal parser: any region where top >= bottom is
        // treated as "reset to full screen".
        assert_eq!(parse_csi_r(&[10, 5], 30), ScrollMargins { top: 1, bottom: 30 });
        assert_eq!(parse_csi_r(&[5, 5], 30), ScrollMargins { top: 1, bottom: 30 });
    }

    #[test]
    fn out_of_grid_falls_back() {
        // bottom > rows must trip the full-screen fallback.
        assert_eq!(parse_csi_r(&[1, 99], 30), ScrollMargins { top: 1, bottom: 30 });
    }
}
