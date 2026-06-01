// -----------------------------------------------------------------------------
// @aphrody-code/m3-tokens — theme-to-tokens
// -----------------------------------------------------------------------------
// Convert a MUI theme object (Material 2, `createTheme`) into a block of M3
// `--md-sys-*` tokens consumable by material-web (`<md-*>`). It is the entry
// point of the MUI → Material 3 migration path.
//
// See the contract: migration/00-CONVENTIONS.md (§5) and the reference docs
//   docs/02-tokens-theming-web.md  +  migration/02-theme-token-migration.md
//
// material-color-utilities (MCU) derives the M3 roles MUI has no equivalent for
// (tertiary, *-container, surface-variant, outline, inverse-*, tonal surfaces)
// from a single source colour.
//
// Fidelity notes M2 → M3 (detailed in the .md):
//  * MUI (M2) lacks "container", "tertiary", "surface-variant", "outline",
//    "inverse-*" and the tonal surfaces. MCU *invents* them from one source
//    colour — they are not designer-picked.
//  * MUI `primary.main` != the M3 "primary" tone. MCU snaps the hue onto a
//    TonalPalette, so the emitted colour can differ slightly from `primary.main`.
//  * MUI handles light/dark via two distinct themes; M3 derives both from one
//    source. The source of each mode is honoured when provided.
// -----------------------------------------------------------------------------

import { themeFromSourceColor, argbFromHex, hexFromArgb } from "@material/material-color-utilities";

// ---------------------------------------------------------------------------
// MUI theme shape (subset we read). Loosely typed: every field is optional.
// ---------------------------------------------------------------------------

export interface MuiColor {
  main?: string;
  contrastText?: string;
}

export interface MuiPalette {
  mode?: "light" | "dark";
  primary?: MuiColor;
  secondary?: MuiColor;
  tertiary?: MuiColor;
  error?: MuiColor;
  background?: { default?: string; paper?: string };
  text?: { primary?: string; secondary?: string };
  divider?: string;
  /** Internal: keep explicit `on-*` colours instead of deriving them. */
  _keepOn?: boolean;
}

export interface MuiTypographyVariant {
  fontFamily?: string;
  fontSize?: number | string;
  lineHeight?: number | string;
  fontWeight?: number | string;
  letterSpacing?: number | string;
}

export interface MuiTypography {
  fontFamily?: string;
  [variant: string]: MuiTypographyVariant | string | undefined;
}

export interface MuiShape {
  borderRadius?: number | string;
}

export interface MuiTheme {
  palette?: MuiPalette;
  typography?: MuiTypography;
  shape?: MuiShape;
  colorSchemes?: { dark?: MuiTheme };
}

export interface ThemeToTokensOptions {
  /** Explicit dark theme (MUI v6 `colorSchemes.dark`, or a 2nd createTheme). */
  darkTheme?: MuiTheme | null;
}

export interface ThemeToTokensResult {
  css: string;
  mcuAvailable: boolean;
  lightRoles: Record<string, string>;
  darkRoles: Record<string, string>;
  typescale: Record<string, string>;
  shape: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Lookup tables
// ---------------------------------------------------------------------------

// MCU Scheme.toJSON() roles (camelCase) -> M3 CSS token (kebab).
const MCU_ROLE_TO_TOKEN: Record<string, string> = {
  primary: "primary",
  onPrimary: "on-primary",
  primaryContainer: "primary-container",
  onPrimaryContainer: "on-primary-container",
  secondary: "secondary",
  onSecondary: "on-secondary",
  secondaryContainer: "secondary-container",
  onSecondaryContainer: "on-secondary-container",
  tertiary: "tertiary",
  onTertiary: "on-tertiary",
  tertiaryContainer: "tertiary-container",
  onTertiaryContainer: "on-tertiary-container",
  error: "error",
  onError: "on-error",
  errorContainer: "error-container",
  onErrorContainer: "on-error-container",
  background: "background",
  onBackground: "on-background",
  surface: "surface",
  onSurface: "on-surface",
  surfaceVariant: "surface-variant",
  onSurfaceVariant: "on-surface-variant",
  outline: "outline",
  outlineVariant: "outline-variant",
  shadow: "shadow",
  scrim: "scrim",
  inverseSurface: "inverse-surface",
  inverseOnSurface: "inverse-on-surface",
  inversePrimary: "inverse-primary",
};

// MUI typography.variant -> M3 typescale <scale>-<size>.
const MUI_VARIANT_TO_TYPESCALE: Record<string, string> = {
  h1: "display-large",
  h2: "display-medium",
  h3: "display-small",
  h4: "headline-large",
  h5: "headline-medium",
  h6: "headline-small",
  subtitle1: "title-medium",
  subtitle2: "title-small",
  body1: "body-large",
  body2: "body-medium",
  button: "label-large",
  caption: "body-small",
  overline: "label-small",
};

// M3 corner values (cf. tokens/versions/v0_192/_md-sys-shape.scss).
const M3_SHAPE_CORNERS: Record<string, number> = {
  "corner-none": 0,
  "corner-extra-small": 4,
  "corner-small": 8,
  "corner-medium": 12,
  "corner-large": 16,
  "corner-extra-large": 28,
  "corner-full": 9999,
};

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

/** Normalise a hex colour (#rgb / #rrggbb) to #rrggbb, or null. */
export function normalizeHex(input: unknown): string | null {
  if (typeof input !== "string") return null;
  let h = input.trim().replace(/^#/, "");
  if (/^[0-9a-fA-F]{3}$/.test(h))
    h = h
      .split("")
      .map((c) => c + c)
      .join("");
  if (/^[0-9a-fA-F]{6}$/.test(h)) return "#" + h.toLowerCase();
  return null;
}

/** Pick black/white text for a background by approximate relative luminance. */
function onColorFor(hex: string): string {
  const n = normalizeHex(hex);
  if (!n) return "#ffffff";
  const r = parseInt(n.slice(1, 3), 16) / 255;
  const g = parseInt(n.slice(3, 5), 16) / 255;
  const b = parseInt(n.slice(5, 7), 16) / 255;
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const L = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  return L > 0.5 ? "#000000" : "#ffffff";
}

// ---------------------------------------------------------------------------
// Colour roles for one mode
// ---------------------------------------------------------------------------

function buildScheme(palette: MuiPalette, mode: "light" | "dark"): Record<string, string> {
  const sourceHex = normalizeHex(palette?.primary?.main) || "#6750a4";
  const roles: Record<string, string> = {};

  const theme = themeFromSourceColor(argbFromHex(sourceHex));
  const scheme = mode === "dark" ? theme.schemes.dark : theme.schemes.light;
  const json = scheme.toJSON() as Record<string, number>;
  for (const [mcuKey, token] of Object.entries(MCU_ROLE_TO_TOKEN)) {
    if (json[mcuKey] != null) roles[token] = hexFromArgb(json[mcuKey]);
  }

  // --- Override with the colours MUI explicitly provides (fidelity) ---
  const ov = (token: string, val: unknown, onToken?: string) => {
    const n = normalizeHex(val);
    if (n) {
      roles[token] = n;
      if (onToken && !palette?._keepOn) roles[onToken] = onColorFor(n);
    }
  };
  ov("primary", palette?.primary?.main, "on-primary");
  ov("secondary", palette?.secondary?.main, "on-secondary");
  ov("tertiary", palette?.tertiary?.main, "on-tertiary");
  ov("error", palette?.error?.main, "on-error");

  // Explicit MUI contrastText: honour the designer's choice.
  const pc = normalizeHex(palette?.primary?.contrastText);
  if (pc) roles["on-primary"] = pc;
  const sc = normalizeHex(palette?.secondary?.contrastText);
  if (sc) roles["on-secondary"] = sc;
  const ec = normalizeHex(palette?.error?.contrastText);
  if (ec) roles["on-error"] = ec;

  // Surface / background / text / divider.
  const bg = normalizeHex(palette?.background?.default);
  const paper = normalizeHex(palette?.background?.paper);
  if (bg) {
    roles.background = bg;
    roles.surface = bg;
  }
  if (paper) roles["surface-container"] = paper;
  const txt = normalizeHex(palette?.text?.primary);
  if (txt) {
    roles["on-background"] = txt;
    roles["on-surface"] = txt;
  }
  const txtSec = normalizeHex(palette?.text?.secondary);
  if (txtSec) roles["on-surface-variant"] = txtSec;
  const divider = normalizeHex(palette?.divider);
  if (divider) roles["outline-variant"] = divider;

  return roles;
}

// ---------------------------------------------------------------------------
// Typescale & shape
// ---------------------------------------------------------------------------

function cssLen(x: number | string): string {
  return typeof x === "number" ? `${x}px` : String(x);
}

function cssLineHeight(lh: number | string, fontSize?: number | string): string {
  if (typeof lh === "number" && lh < 4) {
    if (typeof fontSize === "number") return `${(lh * fontSize).toFixed(2)}px`;
    return `${lh}em`;
  }
  return cssLen(lh);
}

function buildTypescale(typography?: MuiTypography): Record<string, string> {
  if (!typography) return {};
  const out: Record<string, string> = {};
  const globalFamily = typography.fontFamily;

  for (const [variant, token] of Object.entries(MUI_VARIANT_TO_TYPESCALE)) {
    const v = typography[variant] as MuiTypographyVariant | undefined;
    const family = v?.fontFamily || globalFamily;
    if (family) out[`${token}-font`] = family;
    if (v?.fontSize != null) out[`${token}-size`] = cssLen(v.fontSize);
    if (v?.lineHeight != null)
      out[`${token}-line-height`] = cssLineHeight(v.lineHeight, v.fontSize);
    if (v?.fontWeight != null) out[`${token}-weight`] = String(v.fontWeight);
    if (v?.letterSpacing != null) out[`${token}-tracking`] = cssLen(v.letterSpacing);
  }
  return out;
}

function buildShape(shape?: MuiShape): Record<string, string> {
  if (!shape || shape.borderRadius == null) return {};
  const r =
    typeof shape.borderRadius === "number" ? shape.borderRadius : parseFloat(shape.borderRadius);
  if (!isFinite(r)) return {};
  const ratio = r / 4; // 4 = MUI default borderRadius
  const out: Record<string, string> = {};
  for (const [corner, base] of Object.entries(M3_SHAPE_CORNERS)) {
    if (corner === "corner-full") continue; // pill: leave untouched
    out[corner] = `${Math.round(base * ratio)}px`;
  }
  return out;
}

// ---------------------------------------------------------------------------
// CSS emission
// ---------------------------------------------------------------------------

function emitColorBlock(selector: string, roles: Record<string, string>): string {
  const lines = Object.entries(roles).map(([t, v]) => `  --md-sys-color-${t}: ${v};`);
  return `${selector} {\n${lines.join("\n")}\n}`;
}

function emitMiscBlock(
  selector: string,
  typescale: Record<string, string>,
  shape: Record<string, string>,
): string {
  const lines = [
    ...Object.entries(typescale).map(([t, v]) => `  --md-sys-typescale-${t}: ${v};`),
    ...Object.entries(shape).map(([t, v]) => `  --md-sys-shape-${t}: ${v};`),
  ];
  if (lines.length === 0) return "";
  return `${selector} {\n${lines.join("\n")}\n}`;
}

function indent(block: string): string {
  return block
    .split("\n")
    .map((l) => "  " + l)
    .join("\n");
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Convert a MUI theme (palette / typography / shape) into a block of M3
 * `--md-sys-*` token CSS. When a separate dark theme is supplied it feeds the
 * `@media (prefers-color-scheme: dark)` block; otherwise dark is derived from
 * the same source.
 */
export async function muiThemeToTokens(
  lightTheme: MuiTheme,
  options: ThemeToTokensOptions = {},
): Promise<ThemeToTokensResult> {
  const lightPalette: MuiPalette = {
    ...lightTheme.palette,
    mode: "light",
  };
  const lightRoles = buildScheme(lightPalette, "light");

  const typescale = buildTypescale(lightTheme.typography);
  const shape = buildShape(lightTheme.shape);

  const darkTheme = options.darkTheme || lightTheme.colorSchemes?.dark || null;
  const darkPalette: MuiPalette = darkTheme
    ? { ...darkTheme.palette, mode: "dark" }
    : { ...lightPalette, mode: "dark" };
  const darkRoles = buildScheme(darkPalette, "dark");

  const parts: string[] = [];
  parts.push(`/* Generated by @aphrody-code/m3-tokens theme-to-tokens */`);
  parts.push(`:root {\n  color-scheme: light dark;\n}`);
  const misc = emitMiscBlock(":root", typescale, shape);
  parts.push(emitColorBlock(":root", lightRoles));
  if (misc) parts.push(misc);
  parts.push(
    `@media (prefers-color-scheme: dark) {\n${indent(emitColorBlock(":root", darkRoles))}\n}`,
  );

  return {
    css: parts.join("\n\n"),
    mcuAvailable: true,
    lightRoles,
    darkRoles,
    typescale,
    shape,
  };
}
