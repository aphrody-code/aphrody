/** @license SPDX-License-Identifier: Apache-2.0 */

// Minimal M3 token bridge. The live dynamic palette is computed terminal-side
// (Rust crates/m3-tokens), but we keep a small static fallback table so this
// package can resolve color names offline for tests, error messages, and the
// rare case where the renderer needs a hex value upfront.

import type { M3TokenName } from "./types.ts";

// Material 3 dynamic-color baseline (seed #1A73E8, scheme = TonalSpot, dark).
// Pulled from the m3-tokens crate baseline_dark_palette() snapshot. Kept tiny
// on purpose — anything beyond the 24 core roles flows through the terminal.
const FALLBACK_DARK: Readonly<Record<M3TokenName, string>> = Object.freeze({
  primary: "#A8C8FA",
  onPrimary: "#003063",
  primaryContainer: "#00468C",
  onPrimaryContainer: "#D6E3FF",
  secondary: "#BCC7DC",
  onSecondary: "#263141",
  secondaryContainer: "#3C4858",
  onSecondaryContainer: "#D8E3F8",
  tertiary: "#DCBCDF",
  onTertiary: "#3F2844",
  tertiaryContainer: "#573E5C",
  onTertiaryContainer: "#F9D8FB",
  error: "#FFB4AB",
  onError: "#690005",
  errorContainer: "#93000A",
  onErrorContainer: "#FFDAD6",
  background: "#101418",
  onBackground: "#E1E2E8",
  surface: "#101418",
  onSurface: "#E1E2E8",
  surfaceVariant: "#43474E",
  onSurfaceVariant: "#C3C7CF",
  outline: "#8D9199",
  outlineVariant: "#43474E",
  inverseSurface: "#E1E2E8",
  inverseOnSurface: "#2E3035",
  inversePrimary: "#0061A4",
  shadow: "#000000",
  scrim: "#000000",
});

const TOKEN_NAMES = new Set<string>(Object.keys(FALLBACK_DARK));

// Returns true if `value` matches a known M3 token name.
export function isM3Token(value: string): value is M3TokenName {
  return TOKEN_NAMES.has(value);
}

// Resolve a Color string to a CSS-compatible hex value. Tokens go through the
// fallback table; raw hex / rgb strings are passed through unchanged.
export function resolveColor(value: string | undefined): string | undefined {
  if (value === undefined) return undefined;
  if (isM3Token(value)) return FALLBACK_DARK[value];
  return value;
}

// Returns the marker the renderer should embed when forwarding a color token
// to the terminal. Tokens stay symbolic so the terminal can re-resolve them
// against the live palette (which may differ from this fallback).
export function encodeColor(value: string | undefined): string | undefined {
  if (value === undefined) return undefined;
  if (isM3Token(value)) return `m3:${value}`;
  return value;
}

export const M3_FALLBACK_DARK: Readonly<Record<M3TokenName, string>> = FALLBACK_DARK;
