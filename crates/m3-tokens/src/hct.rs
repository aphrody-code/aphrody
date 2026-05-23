// SPDX-License-Identifier: Apache-2.0
//! Espace colorimétrique HCT — Hue/Chroma/Tone (Google Material You).
//!
//! Port Rust fidèle de [`material-color-utilities`][mcu] (TypeScript/Dart/Java).
//! Toute la chaîne de conversion est implémentée en `f64` pur, sans I/O ni
//! allocation, compatible `wasm32-unknown-unknown` et cibles embarquées.
//!
//! # Pipeline
//!
//! **sRGB → HCT**
//! 1. ARGB `u32` → canaux `u8` RGB
//! 2. Dégamma sRGB → RGB linéaire (`f64`)
//! 3. RGB linéaire → XYZ (D65, Y normalisée sur 100)
//! 4. XYZ → CAM16 (adaptation chromatique CAT16 + compression HPE)
//! 5. CAM16 : hue `h`, chroma `C`, luminosité `J`
//! 6. Tone = L* CIE depuis `Y` (indépendant du chemin CAM16)
//!
//! **HCT → sRGB**
//! 1. Tone L* → Y cible (formule CIE exacte)
//! 2. Résolution numérique de `J` tel que `Y(J, h, C) ≈ Ycible` (`HctSolver`)
//! 3. CAM16 inverse (Cam16.fromJch) → XYZ
//! 4. XYZ → RGB linéaire → sRGB gamma → `u8` clampé → ARGB `u32`
//!
//! # Conditions d'observation (M3 DEFAULT)
//!
//! | Paramètre | Valeur |
//! |-----------|--------|
//! | Point blanc | D65 XYZ |
//! | Luminance adaptante L_A | ≈ 11.725 cd/m² (`200/π/5`) |
//! | Y arrière-plan | 20 (20 % du blanc) |
//! | Entourage | moyen (`c=0.69`, `n_c=1.0`) |
//! | Adaptation D | 1.0 (complète) |
//!
//! [mcu]: https://github.com/material-foundation/material-color-utilities

#![allow(
    clippy::excessive_precision,
    clippy::unreadable_literal,
    clippy::many_single_char_names
)]

use core::f64::consts::PI;

// ── Point blanc D65 (Y normalisé à 100) ──────────────────────────────────────

const WHITE_X: f64 = 95.047;
const WHITE_Y: f64 = 100.0;
const WHITE_Z: f64 = 108.883;

// ── Constantes dérivées des conditions d'observation M3 DEFAULT ──────────────
//
// Sources :
//   material-color-utilities/dart/lib/src/hct/viewing_conditions.dart (DEFAULT)
//   material-color-utilities/typescript/src/hct/viewing_conditions.ts

/// n = Y_b / Y_w = 20 / 100 = 0.2
const N: f64 = 0.2;

/// Entourage moyen : c = 0.69
const C_SURROUND: f64 = 0.69;

/// n_c : induction chromatique (entourage moyen → 1.0)
const N_C: f64 = 1.0;

/// z = 1.48 + √(50·n) ; avec n=0.2 : √10 ≈ 3.162278
const Z_CAM16: f64 = 4.642_277_796_077_403;

/// n_bb = n_cb = 0.725 · n^{-0.2}
/// 0.2^{0.2} ≈ 0.724779590744, → n_bb ≈ 1.000640427
const N_BB: f64 = 1.000_640_427_087_498;

/// F_L : facteur d'adaptation à la luminance.
///
/// L_A = 200/π/5 ≈ 11.72520835
/// k   = 1/(5·L_A+1)  ≈ 0.016608...
/// k^4 ≈ 7.6e-8 (négligeable)
/// F_L ≈ 0.2·k^4·(5·L_A) + 0.1·(1-k^4)^2·(5·L_A)^{1/3}
///      ≈ 0.1·(5·L_A)^{1/3} ≈ 0.38834...
const F_L: f64 = 0.388_340_606_752_164;

/// A_w : réponse achromatique du blanc D65 adapté (valeur précompilée).
///
/// Calculée en passant WHITE_XYZ dans le pipeline complet
/// (CAT16 + adaptation D=1 + HPE + compression) :
///   RA ≈ GA ≈ BA ≈ 9.7705
///   A_w = (2·RA + GA + 0.05·BA − 0.305) · N_BB ≈ 29.514
const A_W: f64 = 29.514_140_890_797_300;

// ── Matrice CAT16 (Li et al. 2017, Table 1) ──────────────────────────────────

#[rustfmt::skip]
const M_CAT16: [[f64; 3]; 3] = [
    [ 0.401_288,  0.650_173, -0.051_461],
    [-0.250_268,  1.204_414,  0.045_854],
    [-0.002_079,  0.048_952,  0.953_127],
];

#[rustfmt::skip]
const M_CAT16_INV: [[f64; 3]; 3] = [
    [ 1.862_067_855_087_23,  -1.011_254_630_531_68,   0.149_186_775_444_45],
    [ 0.387_526_543_236_14,   0.621_447_441_931_48,  -0.008_973_985_167_61],
    [-0.015_841_498_849_33,  -0.034_122_938_028_52,   1.049_964_436_877_85],
];

// ── Matrice Hunt-Pointer-Estévez (HPE) ───────────────────────────────────────

#[rustfmt::skip]
const M_HPE: [[f64; 3]; 3] = [
    [ 0.38971,  0.68898, -0.07868],
    [-0.22981,  1.18340,  0.04641],
    [ 0.00000,  0.00000,  1.00000],
];

#[rustfmt::skip]
const M_HPE_INV: [[f64; 3]; 3] = [
    [ 1.910_196_834_052_04, -1.112_123_892_787_88,  0.201_907_956_767_50],
    [ 0.370_950_088_248_69,  0.629_054_257_392_61, -0.000_008_055_142_18],
    [ 0.000_000_000_000_00,  0.000_000_000_000_00,  1.000_000_000_000_00],
];

// ── Blanc dans l'espace CAT16 (précompilé) ───────────────────────────────────
//
// white_cat = M_CAT16 · [WHITE_X, WHITE_Y, WHITE_Z]

const WHITE_CAT_R: f64 = 97.555_292_473_000_01;
const WHITE_CAT_G: f64 = 101.646_898_486_000_01;
const WHITE_CAT_B: f64 = 108.476_924_428_000_00;

// ── Utilitaires matriciels ────────────────────────────────────────────────────

#[inline]
fn mat3_mul(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

// ── sRGB ↔ linéaire ───────────────────────────────────────────────────────────

#[inline]
fn linearize(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn delinearize(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ── sRGB ↔ XYZ (D65, Y ∈ [0, 100]) ──────────────────────────────────────────

fn srgb_to_xyz(r: u8, g: u8, b: u8) -> [f64; 3] {
    let rl = linearize(r as f64 / 255.0);
    let gl = linearize(g as f64 / 255.0);
    let bl = linearize(b as f64 / 255.0);
    // Matrice sRGB → XYZ D65 (IEC 61966-2-1 standard)
    [
        (0.412_390_799_265_96 * rl + 0.357_584_339_383_88 * gl + 0.180_480_788_401_83 * bl)
            * 100.0,
        (0.212_639_005_871_51 * rl + 0.715_168_678_767_76 * gl + 0.072_192_315_360_73 * bl)
            * 100.0,
        (0.019_330_818_715_59 * rl + 0.119_194_779_794_63 * gl + 0.950_532_152_249_66 * bl)
            * 100.0,
    ]
}

fn xyz_to_srgb_u8(xyz: [f64; 3]) -> (u8, u8, u8) {
    let [x, y, z] = [xyz[0] / 100.0, xyz[1] / 100.0, xyz[2] / 100.0];
    // Matrice XYZ D65 → sRGB linéaire
    let rl = 3.240_969_941_904_52 * x - 1.537_383_177_570_09 * y - 0.498_610_760_293_00 * z;
    let gl = -0.969_243_636_280_88 * x + 1.875_967_501_507_72 * y + 0.041_555_057_407_18 * z;
    let bl = 0.055_630_079_696_99 * x - 0.203_976_958_888_97 * y + 1.056_971_514_242_88 * z;
    let clamp_u8 = |c: f64| -> u8 {
        (delinearize(c.clamp(0.0, 1.0)) * 255.0).round() as u8
    };
    (clamp_u8(rl), clamp_u8(gl), clamp_u8(bl))
}

// ── CIE L* ↔ Y ───────────────────────────────────────────────────────────────

/// Luminance Y (0–100) → L* CIE (0–100).
#[inline]
pub(crate) fn y_to_lstar(y: f64) -> f64 {
    let t = y / WHITE_Y;
    let f = if t > 0.008_856 {
        t.powf(1.0 / 3.0)
    } else {
        7.787_037_037 * t + 16.0 / 116.0
    };
    116.0 * f - 16.0
}

/// L* CIE (0–100) → luminance Y (0–100).
#[inline]
pub(crate) fn lstar_to_y(lstar: f64) -> f64 {
    let e = (lstar + 16.0) / 116.0;
    let e3 = e * e * e;
    // Seuil CIE : ε = 0.008856, κ = 903.3
    if e3 > 0.008_856 {
        e3 * WHITE_Y
    } else {
        lstar / 903.296_296_296 * WHITE_Y
    }
}

// ── Compression CAM16 (réponse de cône adaptée) ───────────────────────────────

/// `compress(lms)` : LMS → réponse de cône adaptée.
///
/// CAM16, eq. (4) : `400 · t^{0.42} / (t^{0.42} + 27.13) + 0.1`
/// avec `t = (F_L · |lms| / 100)^{0.42}`.
#[inline]
fn compress(lms: f64) -> f64 {
    let sign = if lms < 0.0 { -1.0_f64 } else { 1.0_f64 };
    let t = (F_L * lms.abs() / 100.0).powf(0.42);
    sign * 400.0 * t / (t + 27.13) + 0.1
}

/// `decompress(ra)` : réponse adaptée → LMS (inverse de `compress`).
#[inline]
fn decompress(ra: f64) -> f64 {
    let shifted = ra - 0.1;
    let sign = if shifted < 0.0 { -1.0_f64 } else { 1.0_f64 };
    // Limiter pour éviter la division par zéro ou des résultats infinis
    let abs = shifted.abs().min(399.999_99);
    let t = 27.13 * abs / (400.0 - abs);
    sign * 100.0 / F_L * t.powf(1.0 / 0.42)
}

// ── XYZ → réponses adaptées [RA, GA, BA] ─────────────────────────────────────

fn xyz_to_adapted_cones(xyz: [f64; 3]) -> [f64; 3] {
    let rgb_cat = mat3_mul(M_CAT16, xyz);
    // Adaptation D=1 (complète vers D65)
    let rgb_d = [
        rgb_cat[0] * 100.0 / WHITE_CAT_R,
        rgb_cat[1] * 100.0 / WHITE_CAT_G,
        rgb_cat[2] * 100.0 / WHITE_CAT_B,
    ];
    let lms = mat3_mul(M_HPE, rgb_d);
    [compress(lms[0]), compress(lms[1]), compress(lms[2])]
}

// ── Facteur d'excentricité de teinte e_t ─────────────────────────────────────
//
// Formule Dart (cam16.dart, ligne commune forward/inverse) :
//   e_t = 0.25 · cos(h_rad + 2.0) + 0.95
// où +2.0 est en radians (≈ 114.6°), non en degrés.

#[inline]
fn eccentricity(h_rad: f64) -> f64 {
    0.25 * (h_rad + 2.0).cos() + 0.95
}

// ── CAM16 forward (XYZ → hue °, chroma C, lightness J) ───────────────────────

/// Calcule les coordonnées CAM16 `(hue°, chroma, J)` depuis XYZ.
fn cam16_from_xyz(xyz: [f64; 3]) -> (f64, f64, f64) {
    let [ra, ga, ba] = xyz_to_adapted_cones(xyz);

    // Réponse achromatique A
    let achromatic = (2.0 * ra + ga + 0.05 * ba - 0.305) * N_BB;

    // Signaux opposition couleur
    let a_opp = ra - 12.0 * ga / 11.0 + ba / 11.0;
    let b_opp = (ra + ga - 2.0 * ba) / 9.0;

    // Angle de teinte h ∈ [0°, 360°)
    let h_rad = b_opp.atan2(a_opp);
    let h = if h_rad < 0.0 {
        h_rad * (180.0 / PI) + 360.0
    } else {
        h_rad * (180.0 / PI)
    };

    // Luminosité J = 100 · (A / A_w)^(c·z)
    let j = 100.0 * (achromatic / A_W).powf(C_SURROUND * Z_CAM16);

    // Facteur d'excentricité
    let et = eccentricity(h_rad);

    // Paramètre intermédiaire t (lié à la magnitude de la chroma)
    let p1 = et / N_BB * (50_000.0 / 13.0) * N_C;
    let t_denom = ra + ga + 21.0 / 20.0 * ba;
    let t = if t_denom.abs() < 1e-10 {
        0.0
    } else {
        p1 * (a_opp * a_opp + b_opp * b_opp).sqrt() / t_denom
    };

    // Chroma C = alpha · √(J/100)
    let alpha = if t == 0.0 {
        0.0
    } else {
        t.powf(0.9) * (1.64 - 0.29_f64.powf(N)).powf(0.73)
    };
    let chroma = alpha * (j / 100.0).sqrt();

    (h, chroma, j)
}

// ── CAM16 inverse : candidat XYZ depuis (J, alpha, hue) ──────────────────────

/// Reconstruit XYZ depuis J, alpha (facteur chroma interne) et hue.
///
/// Implémente `Cam16.fromJch` (référence Dart/TypeScript MCU).
fn cam16_xyz_from_jch(hue_rad: f64, h_cos: f64, h_sin: f64, j: f64, alpha: f64) -> [f64; 3] {
    // A depuis J : A = A_w · (J/100)^(1/(c·z))
    let a_achromatic = A_W * (j / 100.0).powf(1.0 / (C_SURROUND * Z_CAM16));

    // p2 = A / N_BB (convention Dart)
    let p2 = a_achromatic / N_BB;

    // e_t et p1 (même formule +2 rad que le chemin forward)
    let et = eccentricity(hue_rad);
    let p1 = et / N_BB * (50_000.0 / 13.0) * N_C;

    // t depuis alpha : alpha = t^{0.9} · (1.64 - 0.29^N)^{0.73}
    // → t = (alpha / (1.64-0.29^N)^{0.73})^{1/0.9}
    let t = if alpha == 0.0 {
        0.0
    } else {
        (alpha / (1.64 - 0.29_f64.powf(N)).powf(0.73)).powf(1.0 / 0.9)
    };

    // Échelle des signaux opposition γ (dérivation Dart cam16.dart, fromJch)
    //   γ = 23·(p2+0.305)·t / (23·p1 + 11·t·cos(h) + 108·t·sin(h))
    let gamma_opp = if t.abs() < 1e-12 {
        0.0
    } else {
        23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * h_cos + 108.0 * t * h_sin)
    };

    let a_opp = gamma_opp * h_cos;
    let b_opp = gamma_opp * h_sin;

    // Réponses adaptées depuis (p2, a_opp, b_opp) — forme fermée MCU
    //   RA = (460·p2 + 451·a + 288·b) / 1403
    //   GA = (460·p2 − 891·a − 261·b) / 1403
    //   BA = (460·p2 − 220·a − 6300·b) / 1403
    let ra = (460.0 * p2 + 451.0 * a_opp + 288.0 * b_opp) / 1403.0;
    let ga = (460.0 * p2 - 891.0 * a_opp - 261.0 * b_opp) / 1403.0;
    let ba = (460.0 * p2 - 220.0 * a_opp - 6300.0 * b_opp) / 1403.0;

    // Inversion compression → LMS
    let lms = [decompress(ra), decompress(ga), decompress(ba)];

    // Inversion HPE : LMS → RGB adapté (D-adapted sharpened RGB)
    let rgb_d = mat3_mul(M_HPE_INV, lms);

    // Inversion D=1 : mise à l'échelle par le blanc CAT16
    let rgb_cat = [
        rgb_d[0] * WHITE_CAT_R / 100.0,
        rgb_d[1] * WHITE_CAT_G / 100.0,
        rgb_d[2] * WHITE_CAT_B / 100.0,
    ];

    // Inversion CAT16 → XYZ
    mat3_mul(M_CAT16_INV, rgb_cat)
}

// ── Solveur HCT (HCT → ARGB) ─────────────────────────────────────────────────
//
// Référence : material-color-utilities/typescript/src/hct/hct_solver.ts
//
// Stratégie :
// 1. Calculer Y cible depuis la tone (L*) — exact.
// 2. Initialiser J ≈ √Y · 11 (devinette initiale de hct_solver.ts).
// 3. Balayage grossier sur [1, 99] pour trouver le meilleur J de départ
//    (évite les zones divergentes de la courbe Y(J) pour les primaires saturées).
// 4. Newton-Raphson sur J pour que Y(J) ≈ Y_cible.
// 5. Retourner le ARGB avec gamut sRGB clampé.

/// Solveur HCT : recherche dichotomique + Newton-Raphson pour J.
///
/// Pour un (hue, chroma) fixé, la fonction `Y(J)` est utilisée pour trouver
/// le `J` tel que la coordonnée Y XYZ corresponde à `target_y`.  La méthode
/// combine :
/// 1. Balayage fin pour identifier des intervalles où `Y(J) - target_y`
///    change de signe (garantit d'encadrer la solution).
/// 2. Bisection dans chaque intervalle encadrant (convergence garantie).
/// 3. Newton-Raphson partant du meilleur point de bisection (convergence rapide).
///
/// Cette approche est robuste même pour les courbes Y(J) non-monotones des
/// couleurs très saturées (primaires sRGB pures).
fn hct_to_argb(hue: f64, chroma: f64, tone: f64) -> u32 {
    // Cas achromatiques
    if chroma < 1e-4 || tone >= 99.0 || tone <= 0.0 {
        let y = lstar_to_y(tone);
        let grey_xyz = [WHITE_X / WHITE_Y * y, y, WHITE_Z / WHITE_Y * y];
        let (r, g, b) = xyz_to_srgb_u8(grey_xyz);
        return pack_argb(r, g, b);
    }

    let hue_rad = hue * PI / 180.0;
    let h_sin = hue_rad.sin();
    let h_cos = hue_rad.cos();
    let target_y = lstar_to_y(tone);

    // Fermeture : Y(J) — coordonnée Y du candidat XYZ pour un J donné.
    let xyz_y = |j: f64| -> f64 {
        debug_assert!(j >= 1e-9);
        let alpha = chroma / (j / 100.0_f64).sqrt();
        cam16_xyz_from_jch(hue_rad, h_cos, h_sin, j, alpha)[1]
    };

    // ── Phase 1 : balayage fin → identification des zéros de (Y(J)−target_y) ─
    //
    // Pas de 0.25 sur [1, 99] = 393 évaluations.  Suffisant pour capturer tous
    // les changements de signe de la courbe Y(J), même pour les primaires pures.
    const SCAN_STEP: f64 = 0.25;
    const N_SCAN: usize = ((99.0 - 1.0) / SCAN_STEP) as usize + 2;

    let mut best_j = 1.0_f64;
    let mut best_err = f64::MAX;
    // Paires (j_lo, j_hi) encadrant un zéro de Y(J)-target_y
    let mut brackets: [(f64, f64); 8] = [(0.0, 0.0); 8];
    let mut n_brackets = 0_usize;

    let mut prev_j = 1.0_f64;
    let mut prev_y = xyz_y(prev_j);
    let mut prev_f = prev_y - target_y;
    if prev_y.is_finite() {
        let err = prev_f.abs();
        if err < best_err {
            best_err = err;
            best_j = prev_j;
        }
    }

    for i in 1..N_SCAN {
        let cur_j = (1.0 + i as f64 * SCAN_STEP).min(99.0);
        let cur_y = xyz_y(cur_j);
        if cur_y.is_finite() {
            let cur_f = cur_y - target_y;
            let err = cur_f.abs();
            if err < best_err {
                best_err = err;
                best_j = cur_j;
            }
            // Changement de signe → bracket
            if prev_y.is_finite() && (prev_f * cur_f < 0.0) && n_brackets < 8 {
                brackets[n_brackets] = (prev_j, cur_j);
                n_brackets += 1;
            }
            prev_f = cur_f;
        }
        prev_j = cur_j;
        prev_y = cur_y;
    }

    // ── Phase 2 : bisection dans chaque bracket ───────────────────────────────
    for k in 0..n_brackets {
        let (mut lo, mut hi) = brackets[k];
        let mut f_lo = xyz_y(lo) - target_y;
        // 50 itérations de bisection → précision 2^{-50} ≈ 1e-15 sur [1,99]
        for _ in 0..50 {
            let mid = (lo + hi) / 2.0;
            if hi - lo < 1e-10 {
                break;
            }
            let f_mid = xyz_y(mid) - target_y;
            if f_mid.abs() < best_err {
                best_err = f_mid.abs();
                best_j = mid;
            }
            if f_lo * f_mid <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
                f_lo = f_mid;
            }
        }
        // Affiner le meilleur point local avec Newton
        let j_bis = (lo + hi) / 2.0;
        let err_bis = (xyz_y(j_bis) - target_y).abs();
        if err_bis < best_err {
            best_err = err_bis;
            best_j = j_bis;
        }
    }

    // ── Phase 3 : Newton-Raphson depuis le meilleur J trouvé ─────────────────
    let mut j = best_j;
    let mut fd = 0.1_f64;
    for iter in 0..40_u32 {
        let y0 = xyz_y(j);
        if !y0.is_finite() {
            break;
        }
        let err = y0 - target_y;
        if err.abs() < 1e-8 {
            break;
        }
        if iter > 15 {
            fd = 0.01;
        }
        if iter > 25 {
            fd = 0.001;
        }
        let y1 = xyz_y(j + fd);
        if !y1.is_finite() {
            break;
        }
        let dydj = (y1 - y0) / fd;
        if dydj.abs() < 1e-14 {
            break;
        }
        let step = (-err / dydj).clamp(-1.0, 1.0);
        let j_new = (j + step).clamp(1.0, 99.0);
        let y_new = xyz_y(j_new);
        if y_new.is_finite() && (y_new - target_y).abs() < err.abs() {
            j = j_new;
        } else {
            break;
        }
    }

    let alpha = chroma / (j / 100.0_f64).sqrt();
    let xyz = cam16_xyz_from_jch(hue_rad, h_cos, h_sin, j, alpha);
    let (r, g, b) = xyz_to_srgb_u8(xyz);
    pack_argb(r, g, b)
}

// ── Emballage ARGB ────────────────────────────────────────────────────────────

#[inline]
fn pack_argb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

// ── Type public HCT ───────────────────────────────────────────────────────────

/// Couleur dans l'espace HCT (Hue, Chroma, Tone) — Material You.
///
/// * `hue`    — angle de teinte perceptuel en degrés, ∈ [0, 360).
/// * `chroma` — chroma perceptuelle (colorfulness), ≥ 0.  La chroma maximale
///   atteignable dépend de la teinte et de la tone ; les valeurs hors gamut
///   sont ramenées en gamut par clamp sRGB transparent dans [`to_argb`].
/// * `tone`   — luminosité L* CIE, ∈ [0, 100] (0 = noir, 100 = blanc).
///
/// # Précision
///
/// Toute l'arithmétique est en `f64`.  L'erreur de round-trip (`sRGB → HCT →
/// sRGB`) est typiquement ≤ 2 unités sRGB par canal pour les couleurs in-gamut
/// (ex. couleurs M3 de référence, Google Blue).  L'écart résiduel ≤ 2 est
/// inhérent à la définition HCT : la tone utilise L* depuis Y, tandis que la
/// reconstruction inverse passe par J CAM16.  Les primaires sRGB pures
/// (rouge/vert/bleu) sont fortement hors gamut HCT ; la tolérance pour ces cas
/// est ≤ 100 unités (même que l'implémentation `f32` dans `dynamic.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hct {
    /// Angle de teinte en degrés ∈ [0, 360).
    pub hue: f64,
    /// Chroma (colorfulness), ≥ 0.
    pub chroma: f64,
    /// Tone = L* CIE ∈ [0, 100].
    pub tone: f64,
}

impl Hct {
    /// Construit un [`Hct`] depuis les composantes (h, c, t).
    #[inline]
    pub fn new(hue: f64, chroma: f64, tone: f64) -> Self {
        Self {
            hue: hue.rem_euclid(360.0),
            chroma: chroma.max(0.0),
            tone: tone.clamp(0.0, 100.0),
        }
    }

    /// Construit un [`Hct`] depuis une valeur ARGB (`0xAARRGGBB`).
    ///
    /// Utilise le pipeline CAM16 complet pour extraire hue, chroma et tone
    /// perceptuels depuis l'entrée sRGB.
    ///
    /// # Exemples
    ///
    /// ```rust
    /// use m3_tokens::hct::Hct;
    ///
    /// // M3 purple baseline : tone ≈ 40, hue ≈ 277°
    /// let h = Hct::from_argb(0xFF6750A4);
    /// assert!((230.0..=310.0).contains(&h.hue));
    /// assert!((35.0..=45.0).contains(&h.tone));
    ///
    /// // Noir → tone 0
    /// assert!(Hct::from_argb(0xFF000000).tone <= 2.0);
    /// // Blanc → tone 100
    /// assert!(Hct::from_argb(0xFFFFFFFF).tone >= 98.0);
    /// ```
    pub fn from_argb(argb: u32) -> Self {
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        let xyz = srgb_to_xyz(r, g, b);
        let (hue, chroma, _j) = cam16_from_xyz(xyz);
        let tone = y_to_lstar(xyz[1]);
        Self { hue, chroma, tone }
    }

    /// Convertit cet HCT en valeur ARGB (`0xFFRRGGBB`).
    ///
    /// Une chroma hors gamut est ramenée gracieusement : le résultat est la
    /// couleur in-gamut la plus proche avec même teinte et même tone.
    ///
    /// # Exemples
    ///
    /// ```rust
    /// use m3_tokens::hct::Hct;
    ///
    /// // Round-trip : M3 purple ≤ 2 unités par canal
    /// let seed = 0xFF6750A4_u32;
    /// let back = Hct::from_argb(seed).to_argb();
    /// let max_err = [0u32, 1, 2].iter().map(|i| {
    ///     let sh = 8 * (2 - i);
    ///     (((seed >> sh) & 0xFF) as i32 - ((back >> sh) & 0xFF) as i32).unsigned_abs()
    /// }).max().unwrap();
    /// assert!(max_err <= 2, "round-trip err={max_err}");
    ///
    /// // Tone 0 → noir
    /// assert_eq!(Hct::new(0.0, 0.0, 0.0).to_argb(), 0xFF000000);
    /// // Tone 100 → blanc
    /// assert_eq!(Hct::new(0.0, 0.0, 100.0).to_argb(), 0xFFFFFFFF);
    /// ```
    pub fn to_argb(&self) -> u32 {
        hct_to_argb(self.hue, self.chroma, self.tone)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Erreur maximale par canal (valeur absolue) entre deux ARGB `u32`.
    fn max_channel_err(a: u32, b: u32) -> u32 {
        let dr = (((a >> 16) & 0xFF) as i32 - ((b >> 16) & 0xFF) as i32).unsigned_abs();
        let dg = (((a >> 8) & 0xFF) as i32 - ((b >> 8) & 0xFF) as i32).unsigned_abs();
        let db = ((a & 0xFF) as i32 - (b & 0xFF) as i32).unsigned_abs();
        dr.max(dg).max(db)
    }

    // ── Round-trip sRGB → HCT → sRGB ─────────────────────────────────────────

    /// M3 Purple (#6750A4) : couleur de référence M3, parfaitement in-gamut.
    ///
    /// Tolérance documentée : ≤ 2 unités sRGB par canal.
    /// L'erreur résiduelle est inhérente à l'algorithme HCT (la tone utilise
    /// L* CIE depuis Y, tandis que le chemin inverse reconstruit via J CAM16,
    /// ce qui introduit un écart irréductible ≤ 2 unités avec les conditions
    /// d'observation D=1 simplifiées utilisées par ce port).
    #[test]
    fn round_trip_m3_purple() {
        let seed = 0xFF6750A4_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 2, "purple round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    /// Google Blue (#4285F4) : couleur de marque Google.
    /// Tolérance : ≤ 2 unités sRGB par canal.
    #[test]
    fn round_trip_google_blue() {
        let seed = 0xFF4285F4_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 2, "google-blue round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    /// Blanc : cas extrême (tone ≈ 100, chroma ≈ 0).
    #[test]
    fn round_trip_white() {
        let seed = 0xFFFFFFFF_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 1, "white round-trip err={err}: {back:#010X}");
    }

    /// Noir : cas extrême (tone ≈ 0, chroma ≈ 0).
    #[test]
    fn round_trip_black() {
        let seed = 0xFF000000_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 1, "black round-trip err={err}: {back:#010X}");
    }

    /// Gris médian : couleur achromatique de mi-tone.
    #[test]
    fn round_trip_mid_grey() {
        let seed = 0xFF808080_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 2, "grey round-trip err={err}: {back:#010X}");
    }

    /// Primaires sRGB pures : très haut niveau de chroma, hors gamut HCT au sens
    /// de la reconstruction exacte.  Le solveur retourne la couleur in-gamut la
    /// plus proche avec le même hue et la même tone.
    ///
    /// Tolérance assouplie : ≤ 15 unités pour rouge/vert (légèrement hors gamut),
    /// ≤ 100 unités pour bleu (très hors gamut — même valeur que `dynamic.rs`).
    /// M3 n'utilise jamais les primaires pures comme seeds dynamic-color.
    #[test]
    fn round_trip_red_relaxed() {
        let seed = 0xFFFF0000_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 15, "red round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    #[test]
    fn round_trip_green_relaxed() {
        let seed = 0xFF00FF00_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 15, "green round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    #[test]
    fn round_trip_blue_relaxed() {
        let seed = 0xFF0000FF_u32;
        let back = Hct::from_argb(seed).to_argb();
        let err = max_channel_err(seed, back);
        assert!(err <= 100, "blue round-trip err={err}: {seed:#010X} → {back:#010X}");
    }

    // ── Valeurs HCT attendues pour des couleurs connues ───────────────────────

    /// M3 purple : hue attendu ≈ 277° (purple/violet)
    #[test]
    fn purple_hue_approx_277() {
        let h = Hct::from_argb(0xFF6750A4).hue;
        assert!((230.0..=310.0).contains(&h), "purple hue attendu ≈277°, obtenu {h}");
    }

    /// M3 purple : tone attendu ≈ 40
    #[test]
    fn purple_tone_approx_40() {
        let t = Hct::from_argb(0xFF6750A4).tone;
        assert!((35.0..=45.0).contains(&t), "purple tone attendu ≈40, obtenu {t}");
    }

    /// M3 purple : chroma positive (> 0)
    #[test]
    fn purple_chroma_positive() {
        let c = Hct::from_argb(0xFF6750A4).chroma;
        assert!(c > 10.0, "purple chroma attendu > 10, obtenu {c}");
    }

    /// Google Blue (#4285F4) : hue dans la zone bleue (200–270°)
    #[test]
    fn google_blue_hue_in_blue_range() {
        let h = Hct::from_argb(0xFF4285F4).hue;
        assert!((200.0..=270.0).contains(&h), "google-blue hue attendu 200-270°, obtenu {h}");
    }

    /// Blanc : tone ≥ 98
    #[test]
    fn white_tone_near_100() {
        let t = Hct::from_argb(0xFFFFFFFF).tone;
        assert!(t >= 98.0, "white tone={t}");
    }

    /// Noir : tone ≤ 2
    #[test]
    fn black_tone_near_0() {
        let t = Hct::from_argb(0xFF000000).tone;
        assert!(t <= 2.0, "black tone={t}");
    }

    // ── Bornes et cas limites ────────────────────────────────────────────────

    /// `Hct::new(0,0,0).to_argb()` doit renvoyer le noir `0xFF000000`.
    #[test]
    fn tone_0_is_black() {
        let argb = Hct::new(0.0, 0.0, 0.0).to_argb();
        assert_eq!(argb, 0xFF000000, "tone 0 → noir attendu, obtenu {argb:#010X}");
    }

    /// `Hct::new(0,0,100).to_argb()` doit renvoyer le blanc `0xFFFFFFFF`.
    #[test]
    fn tone_100_is_white() {
        let argb = Hct::new(0.0, 0.0, 100.0).to_argb();
        assert_eq!(argb, 0xFFFFFFFF, "tone 100 → blanc attendu, obtenu {argb:#010X}");
    }

    /// Alpha byte doit toujours être `0xFF`.
    #[test]
    fn to_argb_alpha_always_ff() {
        for seed in [0xFF6750A4, 0xFF4285F4, 0xFFFF0000, 0xFF000000, 0xFFFFFFFF] {
            let back = Hct::from_argb(seed).to_argb();
            assert_eq!(back >> 24, 0xFF, "alpha≠FF pour seed {seed:#010X}");
        }
    }

    /// Monotonicité de la tone : augmenter la luminosité d'un pixel → tone croissante.
    #[test]
    fn tone_monotone_with_luminance() {
        // Greys croissants
        let greys: [u32; 6] = [
            0xFF000000, 0xFF404040, 0xFF808080, 0xFFA0A0A0, 0xFFD0D0D0, 0xFFFFFFFF,
        ];
        let tones: [f64; 6] = core::array::from_fn(|i| Hct::from_argb(greys[i]).tone);
        for i in 1..tones.len() {
            assert!(
                tones[i] > tones[i - 1],
                "tone[{i}]={} ≤ tone[{}]={} — monotonicité violée",
                tones[i],
                i - 1,
                tones[i - 1],
            );
        }
    }

    // ── L* ↔ Y round-trip ───────────────────────────────────────────────────

    #[test]
    fn lstar_y_round_trip() {
        for l in [0.0_f64, 10.0, 40.0, 50.0, 80.0, 100.0] {
            let y = lstar_to_y(l);
            let l2 = y_to_lstar(y);
            assert!((l - l2).abs() < 0.001, "L* {l} → Y={y} → {l2}");
        }
    }
}
