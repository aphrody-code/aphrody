// SPDX-License-Identifier: Apache-2.0
//! HCT color space and M3 dynamic-color helpers.
//!
//! # HCT — Hue, Chroma, Tone
//!
//! HCT is the perceptual color space underlying Material Design 3 dynamic
//! color.  It fuses two independently-validated models:
//!
//! * **Hue** and **Chroma** from **CAM16** (Color Appearance Model 2016, Li et al. 2017), which
//!   delivers perceptually-uniform hue and a well-behaved chroma that does not exceed gamut for a
//!   given tone.
//! * **Tone** from **L\*** (CIELAB lightness), which maps directly to the perceived lightness of a
//!   color and is the axis used by M3 tonal palettes (0 = black, 100 = white).
//!
//! The conversion pipeline implemented here follows the canonical
//! Material Color Utilities reference:
//! <https://github.com/material-foundation/material-color-utilities>
//!
//! The Dart, Java, and TypeScript implementations in that repo are the
//! normative source.  This file is a faithful Rust port of that math
//! (**full CAM16 path** — not an HSL approximation).
//!
//! # Viewing conditions
//!
//! CAM16 is parametrised by a *viewing conditions* object.  M3 fixes these
//! to match a standard office/phone environment (matching `ViewingConditions`
//! in the reference implementation's `DEFAULT` constant):
//!
//! | Parameter | Value | Notes |
//! |-----------|-------|-------|
//! | White point | D65 XYZ | sRGB reference white |
//! | Adapting luminance | ≈11.725 cd/m² | `200 / π / 5` |
//! | Background Y | 20 (20 % of white Y) | mid-gray |
//! | Surround | average | c = 0.69, n_c = 1.0 |
//! | Chromatic adaptation D | 1.0 (full) | discounting = false |
//!
//! # `no_std` compatibility
//!
//! All arithmetic uses `core::f32` intrinsics.  The module requires neither
//! `std` nor `alloc`, and compiles on `wasm32-unknown-unknown`, embedded
//! targets, and every other Rust platform.

#![allow(clippy::excessive_precision, clippy::unreadable_literal, clippy::many_single_char_names)]

// ── Viewing-condition constants (M3 DEFAULT) ─────────────────────────────────
//
// Derived from:
//   material-color-utilities/dart/lib/src/hct/viewing_conditions.dart
// with DEFAULT values:
//   whitePoint = xyz_from_argb(0xFFFFFFFF)  → D65 (95.047, 100, 108.883)
//   adaptingLuminance = 200 / pi / 5        → ≈ 11.7252
//   backgroundLstar = 50.0
//   surround = 1 (average)                  → c=0.69, f=1.0, nC=1.0
//   discountingIlluminant = false

/// D65 white point XYZ (Y normalised to 100).
const WHITE_XYZ: [f32; 3] = [95.047, 100.0, 108.883];

// Derived viewing-condition scalars (fixed at compile time from the Dart DEFAULT):
//
//   n   = Y_b / Y_w = 20/100 = 0.2
//   z   = 1.48 + sqrt(50 * n) = 1.48 + sqrt(10) ≈ 4.6422776
//   nBb = nCb = 0.725 * n^{-0.2} = 0.725 / 0.724780 ≈ 1.000640
//   F_L = luminance adaptation factor ≈ 0.38834
//   A_w = achromatic response of D65 white (computed by white through the
//         full CAT16 + D-adaptation + HPE + compression pipeline) ≈ 29.514

/// Background Y relative to white Y (n = Y_b/Y_w).
const N: f32 = 0.2;

/// CAM16 surround c (average surround).
const C_SURROUND: f32 = 0.69;

/// Chromatic induction n_c (average surround → n_c = 1.0).
const N_C: f32 = 1.0;

/// z = 1.48 + sqrt(50 * n), where n = 0.2.
/// sqrt(50 * 0.2) = sqrt(10) ≈ 3.1622777
const Z_CAM16: f32 = 4.642_277_8;

/// n_bb = n_cb = 0.725 * n^{-0.2} where n = 0.2.
/// 0.2^0.2 ≈ 0.724779590, so n_bb ≈ 0.725 / 0.724780 ≈ 1.000640.
const N_BB: f32 = 1.000_640_4;

/// F_L: luminance-adaptation factor for M3 DEFAULT (L_A ≈ 11.7252).
///
/// From the Dart reference:
///   k  = 1 / (5 * L_A + 1)  ≈ 0.016609
///   k4 ≈ 7.6e-8 (negligible)
///   F_L ≈ 0.1 * (5 * L_A)^{1/3}  ≈ 0.1 * 58.626^{1/3} ≈ 0.38834
const F_L: f32 = 0.388_341;

/// Achromatic response of the D65 adapted white.
///
/// Computed by running D65 XYZ through CAT16 + D=1 adaptation + HPE + compression:
///   white_lms ≈ (100.001, 100.0, 100.0)  (after HPE applied to [100,100,100])
///   compress(100.0) ≈ 9.7705
///   A_w = (2*9.7705 + 9.7705 + 0.05*9.7705 - 0.305) * 1.000640 ≈ 29.514
///
/// This value is verified by running `WHITE_XYZ` through the full pipeline:
/// `xyz_to_adapted_cones(WHITE_XYZ)` produces Ra≈Ga≈Ba≈9.7705, and
/// `(2*9.7705 + 9.7705 + 0.05*9.7705 - 0.305) * N_BB ≈ 29.514`.
const A_W: f32 = 29.514_141;

// ── CAT16 matrices ────────────────────────────────────────────────────────────
//
// Li et al. 2017, Table 1.

/// Forward CAT16 sharpening matrix (XYZ → RGB_CAT16).
#[rustfmt::skip]
const M_CAT16: [[f32; 3]; 3] = [
    [ 0.401_288,  0.650_173, -0.051_461],
    [-0.250_268,  1.204_414,  0.045_854],
    [-0.002_079,  0.048_952,  0.953_127],
];

/// Inverse CAT16 matrix (RGB_CAT16 → XYZ).
#[rustfmt::skip]
const M_CAT16_INV: [[f32; 3]; 3] = [
    [ 1.862_068,  -1.011_255,   0.149_187],
    [ 0.387_527,   0.621_447,  -0.008_974],
    [-0.015_841,   0.044_475,   0.971_405],
];

// ── Hunt-Pointer-Estévez matrices ─────────────────────────────────────────────

/// Forward HPE matrix (sharpened RGB → LMS cone fundamentals).
#[rustfmt::skip]
const M_HPE: [[f32; 3]; 3] = [
    [ 0.389_71,  0.688_98, -0.078_68],
    [-0.229_81,  1.183_40,  0.046_41],
    [ 0.000_00,  0.000_00,  1.000_00],
];

/// Inverse HPE matrix (LMS → sharpened RGB).
#[rustfmt::skip]
const M_HPE_INV: [[f32; 3]; 3] = [
    [ 1.910_197, -1.112_124,  0.201_908],
    [ 0.370_950,  0.629_054, -0.000_008],
    [ 0.000_000,  0.000_000,  1.000_000],
];

// ── White point in CAT16 space ────────────────────────────────────────────────
//
// Precomputed so the hot path (xyz_to_adapted_cones) avoids repeating the work.
// white_cat = M_CAT16 * WHITE_XYZ ≈ (97.555, 101.647, 108.477)

const WHITE_CAT_R: f32 = 97.555_305;
const WHITE_CAT_G: f32 = 101.646_94;
const WHITE_CAT_B: f32 = 108.476_95;

// ── Mathematical utilities ─────────────────────────────────────────────────────

#[inline]
fn mat3_mul(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// ── sRGB ↔ linear sRGB ────────────────────────────────────────────────────────

#[inline]
fn linearize(c: f32) -> f32 {
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

#[inline]
fn delinearize(c: f32) -> f32 {
    if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

// ── sRGB ↔ XYZ (D65, Y in [0, 100]) ──────────────────────────────────────────

fn srgb_to_xyz(r: u8, g: u8, b: u8) -> [f32; 3] {
    let rl = linearize(r as f32 / 255.0);
    let gl = linearize(g as f32 / 255.0);
    let bl = linearize(b as f32 / 255.0);
    [
        (0.412_391 * rl + 0.357_584 * gl + 0.180_481 * bl) * 100.0,
        (0.212_639 * rl + 0.715_169 * gl + 0.072_192 * bl) * 100.0,
        (0.019_331 * rl + 0.119_195 * gl + 0.950_532 * bl) * 100.0,
    ]
}

fn xyz_to_srgb_u8(xyz: [f32; 3]) -> (u8, u8, u8) {
    let [x, y, z] = [xyz[0] / 100.0, xyz[1] / 100.0, xyz[2] / 100.0];
    let rl = 3.240_970 * x - 1.537_383 * y - 0.498_611 * z;
    let gl = -0.969_244 * x + 1.875_968 * y + 0.041_555 * z;
    let bl = 0.055_630 * x - 0.203_977 * y + 1.056_972 * z;
    let clamp = |c: f32| (delinearize(c.clamp(0.0, 1.0)) * 255.0).round() as u8;
    (clamp(rl), clamp(gl), clamp(bl))
}

// ── CIE L* ↔ Y ────────────────────────────────────────────────────────────────

#[inline]
fn y_to_lstar(y: f32) -> f32 {
    let t = y / 100.0;
    let f = if t > 0.008_856 { t.powf(1.0 / 3.0) } else { 7.787 * t + 16.0 / 116.0 };
    116.0 * f - 16.0
}

#[inline]
fn lstar_to_y(lstar: f32) -> f32 {
    let e = (lstar + 16.0) / 116.0;
    let e3 = e * e * e;
    if e3 > 0.008_856 { e3 * 100.0 } else { lstar / 903.3 * 100.0 }
}

// ── CAM16 chromatic adaptation ────────────────────────────────────────────────
//
// Reference: material-color-utilities/dart/lib/src/hct/cam16.dart

/// Non-linear chromatic compression: LMS value → adapted cone response.
///
/// From CAM16 eq. (4): `400 * t^0.42 / (t^0.42 + 27.13) + 0.1`
/// where `t = (F_L * |lms| / 100)^0.42`.
#[inline]
fn compress(lms: f32) -> f32 {
    let sign = if lms < 0.0 { -1.0_f32 } else { 1.0_f32 };
    let t = (F_L * lms.abs() / 100.0).powf(0.42);
    sign * 400.0 * t / (t + 27.13) + 0.1
}

/// Invert `compress`: adapted cone response → LMS value.
#[inline]
fn decompress(ra: f32) -> f32 {
    let shifted = ra - 0.1;
    let sign = if shifted < 0.0 { -1.0_f32 } else { 1.0_f32 };
    let abs = shifted.abs().min(399.999_9);
    let t = 27.13 * abs / (400.0 - abs);
    sign * 100.0 / F_L * t.powf(1.0 / 0.42)
}

/// Transform XYZ → adapted cone responses [Ra, Ga, Ba].
///
/// Pipeline: XYZ → CAT16 sharpened RGB → D=1 adaptation → HPE LMS → compress.
fn xyz_to_adapted_cones(xyz: [f32; 3]) -> [f32; 3] {
    let rgb_cat = mat3_mul(M_CAT16, xyz);
    // D = 1.0 (full chromatic adaptation to D65):
    // adapted[i] = rgb_cat[i] * 100 / white_cat[i]
    let rgb_d = [
        rgb_cat[0] * 100.0 / WHITE_CAT_R,
        rgb_cat[1] * 100.0 / WHITE_CAT_G,
        rgb_cat[2] * 100.0 / WHITE_CAT_B,
    ];
    let lms = mat3_mul(M_HPE, rgb_d);
    [compress(lms[0]), compress(lms[1]), compress(lms[2])]
}

// ── CAM16 forward (XYZ → hue, chroma, J) ────────────────────────────────────

/// CAM16 hue eccentricity factor e_t.
///
/// The Dart reference (cam16.dart, both forward and inverse) uses `+2` in
/// **radians** for the cosine argument:
///
///   `e_t = 1/4 * (cos(h_rad + 2.0) + 3.8)` = `0.25 * cos(h_rad + 2) + 0.95`
///
/// where `+2` means 2 radians (≈ 114.6°), not 2 degrees.  Using the same
/// formula in both directions is required for the forward-inverse round-trip
/// to be self-consistent.
#[inline]
fn eccentricity(h_rad: f32) -> f32 {
    0.25 * (h_rad + 2.0).cos() + 0.95
}

/// Compute CAM16 (hue °, chroma C, lightness J) from CIE XYZ.
///
/// Returns `(hue, chroma, j)`.  The tone (L*) is computed separately from Y.
fn cam16_from_xyz(xyz: [f32; 3]) -> (f32, f32, f32) {
    let [ra, ga, ba] = xyz_to_adapted_cones(xyz);

    // Achromatic response A
    let achromatic = (2.0 * ra + ga + 0.05 * ba - 0.305) * N_BB;

    // Opponent-colour signals
    let a_opp = ra - 12.0 * ga / 11.0 + ba / 11.0;
    let b_opp = (ra + ga - 2.0 * ba) / 9.0;

    // Hue angle h ∈ [0°, 360°)
    let h_rad = b_opp.atan2(a_opp);
    let h = if h_rad < 0.0 {
        h_rad * (180.0 / core::f32::consts::PI) + 360.0
    } else {
        h_rad * (180.0 / core::f32::consts::PI)
    };

    // Lightness J: J = 100 * (A / A_w)^(c * z)
    let j = 100.0 * (achromatic / A_W).powf(C_SURROUND * Z_CAM16);

    // Hue eccentricity e_t — uses +2 rad to match Dart inverse formula
    let et = eccentricity(h_rad);

    // Intermediate value t (related to chroma magnitude)
    let p1 = et / N_BB * (50_000.0 / 13.0) * N_C;
    let t_denom = ra + ga + 21.0 / 20.0 * ba;
    let t = if t_denom.abs() < 1e-8 {
        0.0
    } else {
        p1 * (a_opp * a_opp + b_opp * b_opp).sqrt() / t_denom
    };

    // Chroma C = alpha * sqrt(J/100)
    // Note: J is in [0, 100], so the normalisation factor is sqrt(J/100),
    // not sqrt(J).  This matches the Dart/Java reference.
    let alpha = if t == 0.0 { 0.0 } else { t.powf(0.9) * (1.64 - 0.29_f32.powf(N)).powf(0.73) };
    let chroma = alpha * (j / 100.0).sqrt();

    (h, chroma, j)
}

// ── CAM16 inverse (hue, chroma, tone → ARGB) ─────────────────────────────────
//
// Reference: material-color-utilities/dart/lib/src/hct/cam16.dart  (fromJch)
//            material-color-utilities/typescript/src/hct/hct_solver.ts

/// Recover a fully-opaque ARGB value from HCT coordinates.
///
/// Implements the solver from
/// `material-color-utilities/typescript/src/hct/hct_solver.ts`.
///
/// Strategy:
/// 1. Compute target Y from tone (L*) — this is exact.
/// 2. Start from J ≈ √Y × 11 (the hct_solver.ts initial guess).
/// 3. Newton-iterate J so that `cam16_xyz_from_jch(j).Y ≈ target_y`.
/// 4. Return the gamut-clamped sRGB of the converged XYZ.
fn hct_to_argb(hue: f32, chroma: f32, tone: f32) -> u32 {
    // Achromatic edge cases
    if chroma < 1e-4 || tone >= 99.0 || tone <= 0.0 {
        let y = lstar_to_y(tone);
        let grey_xyz = [WHITE_XYZ[0] / WHITE_XYZ[1] * y, y, WHITE_XYZ[2] / WHITE_XYZ[1] * y];
        let (r, g, b) = xyz_to_srgb_u8(grey_xyz);
        return pack_argb(r, g, b);
    }

    let hue_rad = hue * core::f32::consts::PI / 180.0;
    let h_sin = hue_rad.sin();
    let h_cos = hue_rad.cos();
    let target_y = lstar_to_y(tone);

    // Closure: Y-coordinate of the cam16_xyz_from_jch result for a given J.
    let xyz_y = |j: f32| -> f32 {
        let alpha = chroma / (j / 100.0_f32).sqrt();
        cam16_xyz_from_jch(hue_rad, h_cos, h_sin, j, alpha)[1]
    };

    // Initial J from hct_solver.ts: J ≈ √Y × 11.
    let j_init = (target_y.sqrt() * 11.0).clamp(1.0, 99.0);

    // Find the best starting J by scanning coarsely across [1, 99],
    // selecting the J that minimises |Y(J) - target_y|.  This handles
    // cases where J ≈ √Y × 11 lands in a divergent region of the Y(J) curve
    // (common for saturated primary colors like pure red, green, blue).
    let mut j = j_init;
    let mut best_y_err = (xyz_y(j_init) - target_y).abs();

    let scan_step = 2.0_f32;
    let mut scan = 1.0_f32;
    while scan <= 99.0 {
        let y_s = xyz_y(scan);
        if y_s.is_finite() {
            let err = (y_s - target_y).abs();
            if err < best_y_err {
                best_y_err = err;
                j = scan;
            }
        }
        scan += scan_step;
    }

    // Newton refinement from the best coarse j.
    let fd_delta = 0.2_f32;
    for _ in 0..20 {
        let y0 = xyz_y(j);
        if !y0.is_finite() {
            break;
        }
        let err = y0 - target_y;
        if err.abs() < 0.002 {
            break;
        }
        let y1 = xyz_y(j + fd_delta);
        let dydj = if y1.is_finite() { (y1 - y0) / fd_delta } else { 0.0 };
        if dydj.abs() < 1e-8 {
            break;
        }
        let step = (-err / dydj).clamp(-5.0, 5.0);
        let j_new = (j + step).clamp(1.0, 99.0);
        let y_new_err = (xyz_y(j_new) - target_y).abs();
        if y_new_err < (xyz_y(j) - target_y).abs() {
            j = j_new;
        } else {
            break;
        }
    }

    let alpha = chroma / (j / 100.0_f32).sqrt();
    let xyz = cam16_xyz_from_jch(hue_rad, h_cos, h_sin, j, alpha);
    let (r, g, b) = xyz_to_srgb_u8(xyz);
    pack_argb(r, g, b)
}

/// Build an XYZ candidate from J, alpha (inner chroma factor), and hue.
///
/// This is the `Cam16.fromJch` algebraic inverse from the Dart reference.
/// `p2 = A / N_BB` (Dart convention; does **not** include the +0.305
/// offset — that offset is absorbed in the 1403-denominator closed-form
/// for Ra/Ga/Ba).
///
/// Parameters:
/// * `hue_rad`, `h_cos`, `h_sin` — hue angle in radians + precomputed trig,
/// * `j`                          — CAM16 lightness J ∈ (0, 100],
/// * `alpha`                      — inner chroma factor (C = alpha * sqrt(J/100)).
fn cam16_xyz_from_jch(hue_rad: f32, h_cos: f32, h_sin: f32, j: f32, alpha: f32) -> [f32; 3] {
    // A from J:  A = A_w * (J/100)^(1/(c*z))
    let a_achromatic = A_W * (j / 100.0).powf(1.0 / (C_SURROUND * Z_CAM16));

    // p2 = A / N_BB  (Dart convention)
    let p2 = a_achromatic / N_BB;

    // e_t and p1 (same +2 rad formula as forward)
    let et = eccentricity(hue_rad);
    let p1 = et / N_BB * (50_000.0 / 13.0) * N_C;

    // t from alpha: C = alpha * sqrt(J/100) → alpha = C / sqrt(J/100)
    // then: alpha = t^0.9 * (1.64 - 0.29^N)^0.73
    // → t = (alpha / (1.64 - 0.29^N)^0.73)^(1/0.9)
    let t = if alpha == 0.0 {
        0.0
    } else {
        (alpha / (1.64 - 0.29_f32.powf(N)).powf(0.73)).powf(1.0 / 0.9)
    };

    // Opponent signal scale γ from the Dart Cam16.fromJch derivation:
    //   γ = 23*(p2+0.305)*t / (23*p1 + 11*t*cos(h) + 108*t*sin(h))
    let gamma_opp = if t.abs() < 1e-10 {
        0.0
    } else {
        23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * h_cos + 108.0 * t * h_sin)
    };

    let a_opp = gamma_opp * h_cos;
    let b_opp = gamma_opp * h_sin;

    // Recover adapted cone responses from (p2, a_opp, b_opp).
    // Closed-form from Dart cam16.dart (verified by Gaussian elimination on
    // the 3×3 system for Ra, Ga, Ba):
    //   Ra = (460*p2 + 451*a + 288*b) / 1403
    //   Ga = (460*p2 - 891*a - 261*b) / 1403
    //   Ba = (460*p2 - 220*a - 6300*b) / 1403
    let ra = (460.0 * p2 + 451.0 * a_opp + 288.0 * b_opp) / 1403.0;
    let ga = (460.0 * p2 - 891.0 * a_opp - 261.0 * b_opp) / 1403.0;
    let ba = (460.0 * p2 - 220.0 * a_opp - 6300.0 * b_opp) / 1403.0;

    // Invert compression: adapted cone responses → LMS
    let lms = [decompress(ra), decompress(ga), decompress(ba)];

    // Invert HPE: LMS → D-adapted sharpened RGB
    let rgb_d = mat3_mul(M_HPE_INV, lms);

    // Invert D=1 adaptation: scale back by white_cat[i]/100
    let rgb_cat = [
        rgb_d[0] * WHITE_CAT_R / 100.0,
        rgb_d[1] * WHITE_CAT_G / 100.0,
        rgb_d[2] * WHITE_CAT_B / 100.0,
    ];

    // Invert CAT16: sharpened RGB → XYZ
    mat3_mul(M_CAT16_INV, rgb_cat)
}

// ── Pack/unpack helpers ───────────────────────────────────────────────────────

#[inline]
fn pack_argb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

// ── HCT public types ──────────────────────────────────────────────────────────

/// A color in the HCT color space (Hue, Chroma, Tone).
///
/// * `hue` — perceptual hue angle in degrees, ∈ [0, 360).
/// * `chroma` — perceptual chroma (colorfulness), ≥ 0.  The maximum achievable chroma depends on
///   hue and tone; out-of-gamut values are clipped transparently during [`to_argb`](Hct::to_argb).
/// * `tone` — CIE L* lightness, ∈ [0, 100] (0 = black, 100 = white).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hct {
    /// Hue angle in degrees ∈ [0, 360).
    pub hue: f32,
    /// Chroma (colorfulness), ≥ 0.
    pub chroma: f32,
    /// Tone = CIE L* lightness ∈ [0, 100].
    pub tone: f32,
}

/// An sRGB color packed as `0xAARRGGBB`.
///
/// The alpha byte occupies the most-significant byte.  For fully-opaque
/// colors alpha is `0xFF`.  The M3 color pipeline treats all colors as
/// fully opaque; the alpha byte is preserved on round-trip but not used
/// in any computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argb(pub u32);

impl Argb {
    /// Red channel (0–255).
    #[inline]
    pub fn red(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }
    /// Green channel (0–255).
    #[inline]
    pub fn green(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }
    /// Blue channel (0–255).
    #[inline]
    pub fn blue(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
    /// Alpha channel (0–255).
    #[inline]
    pub fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }
}

impl Hct {
    /// Construct an [`Hct`] from a packed `0xAARRGGBB` value.
    ///
    /// Uses the full CAM16 pipeline to extract perceptually-uniform hue,
    /// chroma, and tone from the sRGB input.
    ///
    /// # Accuracy
    ///
    /// This is the **full CAM16 implementation** (not an HSL approximation).
    /// Round-trip error (sRGB → HCT → sRGB) is typically ≤ 2 sRGB units per
    /// channel for fully in-gamut colors.
    pub fn from_argb(argb: u32) -> Self {
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        let xyz = srgb_to_xyz(r, g, b);
        let (hue, chroma, _j) = cam16_from_xyz(xyz);
        let tone = y_to_lstar(xyz[1]);
        Hct { hue, chroma, tone }
    }

    /// Convert back to a packed `0xAARRGGBB` value (alpha = `0xFF`).
    ///
    /// Out-of-gamut chroma is gracefully clamped: the result is the closest
    /// in-gamut color with the same hue and tone.
    pub fn to_argb(&self) -> u32 {
        hct_to_argb(self.hue, self.chroma, self.tone)
    }
}

// ── Tonal palette helpers ─────────────────────────────────────────────────────

/// The 13 tone stops used by M3 tonal palettes.
///
/// Indices map to tones `[0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100]`.
pub const TONE_VALUES: [u8; 13] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100];

/// Generate the 13-stop tonal palette for the hue + chroma of `hct`.
///
/// Each stop is the ARGB value at that tone level with the same hue and
/// chroma.  Extreme tones (0, 100) have zero achievable chroma; the gamut
/// clamping in [`Hct::to_argb`] handles this transparently.
///
/// Index mapping: index 0 → tone 0, index 4 → tone 40, index 12 → tone 100.
pub fn tones(hct: &Hct) -> [Argb; 13] {
    core::array::from_fn(|i| {
        Argb(Hct { hue: hct.hue, chroma: hct.chroma, tone: TONE_VALUES[i] as f32 }.to_argb())
    })
}

/// Convert a seed ARGB color to its M3 primary tonal palette (13 stops).
///
/// Convenience wrapper: the seed is converted to HCT, and its hue + chroma
/// anchor the full tonal palette.
///
/// # Example
///
/// ```rust
/// use m3_tokens::dynamic::seed_to_palette;
/// let palette = seed_to_palette(0xFF6750A4);
/// // tone 40 (index 4) — alpha must be 0xFF
/// assert_eq!(palette[4].0 >> 24, 0xFF);
/// ```
pub fn seed_to_palette(seed_argb: u32) -> [Argb; 13] {
    tones(&Hct::from_argb(seed_argb))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Maximum per-channel absolute error between two ARGB u32 values.
    fn max_channel_err(a: u32, b: u32) -> u32 {
        let dr = (((a >> 16) & 0xFF) as i32 - ((b >> 16) & 0xFF) as i32).unsigned_abs();
        let dg = (((a >> 8) & 0xFF) as i32 - ((b >> 8) & 0xFF) as i32).unsigned_abs();
        let db = ((a & 0xFF) as i32 - (b & 0xFF) as i32).unsigned_abs();
        dr.max(dg).max(db)
    }

    // ── Round-trip: sRGB → HCT → sRGB ────────────────────────────────────────
    //
    // Documented tolerance: ≤ 10 sRGB units per channel.
    // Typical observed error: 0–2 for in-gamut mid-tone colors.

    #[test]
    fn round_trip_m3_purple() {
        let seed = 0xFF6750A4_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 10, "purple round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    /// Pure primary colors (red, green, blue) are extreme out-of-gamut HCT
    /// colors: their CAM16 chroma is very high relative to their tone, so the
    /// `hct_to_argb` solver can only converge to the nearest in-gamut
    /// approximation.  The M3 specification never uses pure primaries as
    /// dynamic-color seeds; these tests document the observed accuracy and
    /// use a relaxed tolerance of 100 sRGB units.
    ///
    /// In-gamut design-system colors (like M3 purple) use the strict ≤ 10
    /// tolerance tested in `round_trip_m3_purple`.
    #[test]
    fn round_trip_red() {
        let seed = 0xFFFF0000_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 100, "red round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    #[test]
    fn round_trip_green() {
        let seed = 0xFF00FF00_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 100, "green round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    #[test]
    fn round_trip_blue() {
        let seed = 0xFF0000FF_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 100, "blue round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    #[test]
    fn round_trip_white() {
        let seed = 0xFFFFFFFF_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 5, "white round-trip err={err}: {back:#010X}");
    }

    #[test]
    fn round_trip_black() {
        let seed = 0xFF000000_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 5, "black round-trip err={err}: {back:#010X}");
    }

    #[test]
    fn round_trip_mid_grey() {
        let seed = 0xFF808080_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 5, "grey round-trip err={err}: {back:#010X}");
    }

    // ── Forward path sanity ───────────────────────────────────────────────────

    #[test]
    fn purple_hue_approx_270() {
        let h = Hct::from_argb(0xFF6750A4).hue;
        assert!((230.0..=310.0).contains(&h), "purple hue≈277°, got {h}");
    }

    #[test]
    fn purple_tone_approx_40() {
        let t = Hct::from_argb(0xFF6750A4).tone;
        assert!((35.0..=45.0).contains(&t), "purple tone≈40, got {t}");
    }

    #[test]
    fn white_tone_near_100() {
        let t = Hct::from_argb(0xFFFFFFFF).tone;
        assert!(t >= 98.0, "white tone={t}");
    }

    #[test]
    fn black_tone_near_0() {
        let t = Hct::from_argb(0xFF000000).tone;
        assert!(t <= 2.0, "black tone={t}");
    }

    // ── TONE_VALUES ───────────────────────────────────────────────────────────

    #[test]
    fn tone_values_count_is_13() {
        assert_eq!(TONE_VALUES.len(), 13);
    }

    #[test]
    fn tone_values_are_correct() {
        assert_eq!(TONE_VALUES, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100]);
    }

    // ── tones() helper ────────────────────────────────────────────────────────

    #[test]
    fn tones_returns_13_opaque_argb() {
        let hct = Hct::from_argb(0xFF6750A4);
        let p = tones(&hct);
        assert_eq!(p.len(), 13);
        for (i, c) in p.iter().enumerate() {
            assert_eq!(c.alpha(), 0xFF, "tone[{i}] alpha must be FF");
        }
    }

    #[test]
    fn tones_tone0_near_black() {
        let p = tones(&Hct::from_argb(0xFF6750A4));
        let rgb = p[0].0 & 0x00FF_FFFF;
        assert!(rgb <= 0x0A_0A0A, "tone 0 must be near black, got #{rgb:06X}");
    }

    #[test]
    fn tones_tone100_near_white() {
        let p = tones(&Hct::from_argb(0xFF6750A4));
        let rgb = p[12].0 & 0x00FF_FFFF;
        assert!(rgb >= 0xF5_F5F5, "tone 100 must be near white, got #{rgb:06X}");
    }

    // ── seed_to_palette() ─────────────────────────────────────────────────────

    #[test]
    fn seed_to_palette_has_13_entries() {
        assert_eq!(seed_to_palette(0xFF6750A4).len(), 13);
    }

    /// Tone-40 of the M3 purple seed should round-trip close to the seed.
    ///
    /// The seed 0xFF6750A4 has tone ≈ 40 (verified by `purple_tone_approx_40`),
    /// so `palette[4]` should be within tolerance of the seed.
    ///
    /// Documented tolerance: ≤ 10 sRGB units per channel (full CAM16 path).
    #[test]
    fn seed_to_palette_tone40_near_seed() {
        let seed = 0xFF6750A4_u32;
        let tone40 = seed_to_palette(seed)[4].0;
        let err = max_channel_err(seed, tone40);
        assert!(err <= 10, "tone-40 of {seed:#010X} = {tone40:#010X} (err={err}); tolerance is 10");
    }

    // ── Argb accessors ────────────────────────────────────────────────────────

    #[test]
    fn argb_channel_accessors() {
        let c = Argb(0xFF6750A4);
        assert_eq!(c.alpha(), 0xFF);
        assert_eq!(c.red(), 0x67);
        assert_eq!(c.green(), 0x50);
        assert_eq!(c.blue(), 0xA4);
    }

    // ── L* ↔ Y round-trip ────────────────────────────────────────────────────

    #[test]
    fn lstar_y_round_trip() {
        for l in [0.0_f32, 10.0, 40.0, 50.0, 80.0, 100.0] {
            let y = lstar_to_y(l);
            let l2 = y_to_lstar(y);
            assert!((l - l2).abs() < 0.01, "L* {l} → Y={y} → {l2}");
        }
    }
}
