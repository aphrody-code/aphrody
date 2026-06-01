// -----------------------------------------------------------------------------
// @aphrody-code/m3-tokens — dynamic-color
// -----------------------------------------------------------------------------
// Runtime "Material You" dynamic colour: derive the full set of M3
// `--md-sys-color-*` roles from ANY single seed colour, for light and dark, and
// apply them live. This is the signature Material 3 capability — a host app can
// re-theme every `<md-*>` element (and every `@aphrody-code/m3-react` wrapper) from
// one colour, with no rebuild — that MUI has no native equivalent for.
//
// Built on @material/material-color-utilities (7 scheme variants: tonalSpot,
// content, fidelity, expressive, vibrant, neutral, monochrome).
// Pure functions are environment-agnostic and unit-testable under bun;
// only `applyDynamicColor` touches the DOM.
// -----------------------------------------------------------------------------

import {
  argbFromHex,
  hexFromArgb,
  Hct,
  SchemeTonalSpot,
  SchemeContent,
  SchemeFidelity,
  SchemeExpressive,
  SchemeVibrant,
  SchemeNeutral,
  SchemeMonochrome,
  MaterialDynamicColors,
  type DynamicColor,
  type DynamicScheme,
} from "@material/material-color-utilities";

// ---------------------------------------------------------------------------
// Variant type
// ---------------------------------------------------------------------------

/**
 * MCU colour scheme variants, exposed as a string union for ergonomic use.
 *
 * | Variant        | Character                                                  |
 * |----------------|------------------------------------------------------------|
 * | `tonalSpot`    | Default M3 (Android 12+): low–medium chroma, complementary tertiary |
 * | `content`      | Source colour in `primaryContainer`, high fidelity to input |
 * | `fidelity`     | Like content but tertiary from temperature complement       |
 * | `expressive`   | Detached from source — bold, diverse palette               |
 * | `vibrant`      | Maximum chroma at every tonal position                     |
 * | `neutral`      | Near-greyscale, very low chroma                             |
 * | `monochrome`   | Pure greyscale — zero chroma                               |
 */
export type SchemeVariant =
  | "tonalSpot"
  | "content"
  | "fidelity"
  | "expressive"
  | "vibrant"
  | "neutral"
  | "monochrome";

/** Options shared by every scheme-deriving function. */
export interface SchemeOptions {
  /** Resolve the dark variant of the scheme. */
  dark?: boolean;
  /**
   * Contrast level in `[-1, 1]` (0 = standard, 0.5 = medium, 1 = maximum /
   * AAA-grade). Negative values reduce contrast below the M3 standard.
   * Alias: `contrast` (legacy, same field).
   */
  contrastLevel?: number;
  /**
   * @deprecated Use `contrastLevel`. Kept for backwards compatibility.
   * When both are provided, `contrastLevel` takes precedence.
   */
  contrast?: number;
  /**
   * MCU colour scheme variant (default: `"tonalSpot"`).
   * All variants produce the same ~47 `--md-sys-color-*` roles; only the
   * palette relationships and chroma differ.
   */
  variant?: SchemeVariant;
}

/** A map of `--md-sys-color-*` custom-property names to hex colours. */
export type ColorRoleMap = Record<string, string>;

// ---------------------------------------------------------------------------
// Role list
// ---------------------------------------------------------------------------

// The MaterialDynamicColors roles that map 1:1 onto `--md-sys-color-*` tokens.
// (Palette key colours and non-colour spec fields are intentionally excluded.)
export const ROLES = [
  "background",
  "onBackground",
  "surface",
  "surfaceDim",
  "surfaceBright",
  "surfaceContainerLowest",
  "surfaceContainerLow",
  "surfaceContainer",
  "surfaceContainerHigh",
  "surfaceContainerHighest",
  "onSurface",
  "surfaceVariant",
  "onSurfaceVariant",
  "inverseSurface",
  "inverseOnSurface",
  "outline",
  "outlineVariant",
  "shadow",
  "scrim",
  "surfaceTint",
  "primary",
  "onPrimary",
  "primaryContainer",
  "onPrimaryContainer",
  "inversePrimary",
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
  "primaryFixed",
  "primaryFixedDim",
  "onPrimaryFixed",
  "onPrimaryFixedVariant",
  "secondaryFixed",
  "secondaryFixedDim",
  "onSecondaryFixed",
  "onSecondaryFixedVariant",
  "tertiaryFixed",
  "tertiaryFixedDim",
  "onTertiaryFixed",
  "onTertiaryFixedVariant",
] as const;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

const dynamicColors = MaterialDynamicColors as unknown as Record<string, DynamicColor | undefined>;

/** `onPrimaryContainer` -> `on-primary-container`. */
function kebab(role: string): string {
  return role.replace(/([A-Z])/g, "-$1").toLowerCase();
}

/** Resolve the effective contrast level, preferring `contrastLevel` over `contrast`. */
function resolveContrast(opts: SchemeOptions): number {
  if (opts.contrastLevel !== undefined) return opts.contrastLevel;
  if (opts.contrast !== undefined) return opts.contrast;
  return 0;
}

/** Build the MCU DynamicScheme for the requested variant. */
function buildScheme(
  hct: Hct,
  isDark: boolean,
  contrastLevel: number,
  variant: SchemeVariant,
): DynamicScheme {
  switch (variant) {
    case "content":
      return new SchemeContent(hct, isDark, contrastLevel);
    case "fidelity":
      return new SchemeFidelity(hct, isDark, contrastLevel);
    case "expressive":
      return new SchemeExpressive(hct, isDark, contrastLevel);
    case "vibrant":
      return new SchemeVibrant(hct, isDark, contrastLevel);
    case "neutral":
      return new SchemeNeutral(hct, isDark, contrastLevel);
    case "monochrome":
      return new SchemeMonochrome(hct, isDark, contrastLevel);
    case "tonalSpot":
    default:
      return new SchemeTonalSpot(hct, isDark, contrastLevel);
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Resolve every M3 colour role for one mode from a seed colour.
 *
 * Backwards-compatible: calling `schemeFromSeed(hex)` or
 * `schemeFromSeed(hex, { dark, contrast })` still works exactly as before.
 *
 * @param seedHex Source colour, e.g. `"#6750A4"`.
 * @param opts    Optional: `dark`, `contrastLevel` (or legacy `contrast`), `variant`.
 * @returns `{ "--md-sys-color-primary": "#…", … }` (~47 roles).
 */
export function schemeFromSeed(seedHex: string, opts: SchemeOptions = {}): ColorRoleMap {
  const dark = opts.dark ?? false;
  const contrastLevel = resolveContrast(opts);
  const variant = opts.variant ?? "tonalSpot";

  const hct = Hct.fromInt(argbFromHex(seedHex));
  const scheme = buildScheme(hct, dark, contrastLevel, variant);

  const out: ColorRoleMap = {};
  for (const role of ROLES) {
    const dynamicColor = dynamicColors[role];
    if (!dynamicColor) continue;
    out[`--md-sys-color-${kebab(role)}`] = hexFromArgb(dynamicColor.getArgb(scheme));
  }
  return out;
}

/** Serialise a role map to CSS custom-property declarations. */
function declarations(vars: ColorRoleMap, indent = "  "): string {
  return Object.entries(vars)
    .map(([k, v]) => `${indent}${k}: ${v};`)
    .join("\n");
}

/**
 * Full stylesheet text: `:root` (light) + `:root[data-theme="dark"]` (dark),
 * ready to inline into a `<style>` for SSR or static theming.
 */
export function cssFromSeed(seedHex: string, opts: SchemeOptions = {}): string {
  const light = schemeFromSeed(seedHex, { ...opts, dark: false });
  const dark = schemeFromSeed(seedHex, { ...opts, dark: true });
  return (
    `:root {\n  color-scheme: light;\n${declarations(light)}\n}\n\n` +
    `:root[data-theme="dark"] {\n  color-scheme: dark;\n${declarations(dark)}\n}\n`
  );
}

/** Options for {@link applyDynamicColor}. */
export interface ApplyOptions extends SchemeOptions {
  /** Element whose inline style receives the roles (default `<html>`). */
  target?: HTMLElement | null;
}

/**
 * Apply a seed's roles for one mode directly onto an element's inline style
 * (overriding any stylesheet `:root` values). Browser-only; a no-op in SSR.
 */
export function applyDynamicColor(seedHex: string, opts: ApplyOptions = {}): void {
  const target = opts.target ?? (typeof document !== "undefined" ? document.documentElement : null);
  if (!target) return;
  const vars = schemeFromSeed(seedHex, opts);
  for (const [k, v] of Object.entries(vars)) target.style.setProperty(k, v);
}

/** Remove every `--md-sys-color-*` role this module may have set inline. */
export function clearDynamicColor(target?: HTMLElement | null): void {
  const el = target ?? (typeof document !== "undefined" ? document.documentElement : null);
  if (!el) return;
  for (const role of ROLES) el.style.removeProperty(`--md-sys-color-${kebab(role)}`);
}
