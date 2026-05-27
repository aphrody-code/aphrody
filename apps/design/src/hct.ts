// SPDX-License-Identifier: Apache-2.0
//! HCT-inspired tonal palette generator for Material Design 3 theme compilation.

import type {M3Theme, TonalPalette} from './types.ts';

// Helper to convert hex to HSL
function hexToHsl(hex: string): {h: number; s: number; l: number} {
  let r = 0,
    g = 0,
    b = 0;
  // Parse hex
  if (hex.length === 4) {
    r = parseInt(hex[1] + hex[1], 16);
    g = parseInt(hex[2] + hex[2], 16);
    b = parseInt(hex[3] + hex[3], 16);
  } else if (hex.length === 7) {
    r = parseInt(hex.substring(1, 3), 16);
    g = parseInt(hex.substring(3, 5), 16);
    b = parseInt(hex.substring(5, 7), 16);
  }
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      case b:
        h = (r - g) / d + 4;
        break;
    }
    h /= 6;
  }
  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100),
  };
}

// Helper to convert HSL to Hex
function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const y = k(n);
    const val = l - a * Math.max(-1, Math.min(y - 3, 9 - y, 1));
    return Math.round(255 * val)
      .toString(16)
      .padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`.toUpperCase();
}

/**
 * Generates a complete Material Design 3 theme based on a seed color.
 * Leverages HCT (Hue, Chroma, Tone) principles approximated in HSL.
 */
export function generateTheme(seedHex: string): M3Theme {
  const seedHsl = hexToHsl(seedHex);

  const toneSteps = [
    0, 6, 10, 12, 20, 22, 30, 40, 50, 60, 70, 80, 90, 94, 95, 98, 99, 100,
  ];

  const createPalette = (
    name: string,
    hueShift: number,
    saturationMultiplier: number,
  ): TonalPalette => {
    const tones: {[tone: number]: string} = {};
    const hue = (seedHsl.h + hueShift) % 360;

    for (const tone of toneSteps) {
      // Saturation drops as tone reaches extremums (0 and 100), typical of HCT profiles
      const sDrop =
        tone === 0 || tone === 100 ? 0 : Math.sin((tone / 100) * Math.PI);
      const saturation = Math.min(
        100,
        Math.round(seedHsl.s * saturationMultiplier * sDrop),
      );
      tones[tone] = hslToHex(hue, saturation, tone);
    }

    return {name, tones};
  };

  // Generate the 6 core tonal palettes
  const primary = createPalette('primary', 0, 1.0);
  const secondary = createPalette('secondary', 15, 0.4); // slightly shifted, less saturated
  const tertiary = createPalette('tertiary', 60, 0.6); // shifted further
  const error = createPalette('error', 0, 0.9); // custom warning red

  // Set custom error hue (standard red around 0-10 degrees)
  error.tones = {};
  for (const tone of toneSteps) {
    const sDrop =
      tone === 0 || tone === 100 ? 0 : Math.sin((tone / 100) * Math.PI);
    error.tones[tone] = hslToHex(10, Math.round(90 * sDrop), tone);
  }

  // Neutrals are desaturated greys
  const neutral = createPalette('neutral', 0, 0.05);
  const neutralVariant = createPalette('neutral-variant', 0, 0.12);

  // Compile CSS Custom Properties
  let css = `:root {\n  /* Seed Color: ${seedHex} */\n`;
  const palettes = {
    primary,
    secondary,
    tertiary,
    error,
    neutral,
    neutralVariant,
  };

  for (const [key, palette] of Object.entries(palettes)) {
    const prefix = key === 'neutralVariant' ? 'neutral-variant' : key;
    css += `  /* Tonal Palette: ${palette.name} */\n`;
    for (const tone of toneSteps) {
      css += `  --md-ref-palette-${prefix}-${tone}: ${palette.tones[tone]};\n`;
    }
  }

  // Add system token mappings for Light theme as default
  css += `\n  /* System Tokens - Light Mode */\n`;
  css += `  --md-sys-color-primary: var(--md-ref-palette-primary-40);\n`;
  css += `  --md-sys-color-on-primary: var(--md-ref-palette-primary-100);\n`;
  css += `  --md-sys-color-primary-container: var(--md-ref-palette-primary-90);\n`;
  css += `  --md-sys-color-on-primary-container: var(--md-ref-palette-primary-10);\n`;

  css += `  --md-sys-color-secondary: var(--md-ref-palette-secondary-40);\n`;
  css += `  --md-sys-color-on-secondary: var(--md-ref-palette-secondary-100);\n`;
  css += `  --md-sys-color-secondary-container: var(--md-ref-palette-secondary-90);\n`;
  css += `  --md-sys-color-on-secondary-container: var(--md-ref-palette-secondary-10);\n`;

  css += `  --md-sys-color-surface: var(--md-ref-palette-neutral-98);\n`;
  css += `  --md-sys-color-on-surface: var(--md-ref-palette-neutral-10);\n`;
  css += `  --md-sys-color-surface-container: var(--md-ref-palette-neutral-94);\n`;
  css += `  --md-sys-color-surface-container-highest: var(--md-ref-palette-neutral-90);\n`;
  css += `  --md-sys-color-outline: var(--md-ref-palette-neutral-variant-50);\n`;
  css += `  --md-sys-color-outline-variant: var(--md-ref-palette-neutral-variant-80);\n`;

  // Dark mode overrides
  css += `}\n\n@media (prefers-color-scheme: dark) {\n  :root {\n`;
  css += `    --md-sys-color-primary: var(--md-ref-palette-primary-80);\n`;
  css += `    --md-sys-color-on-primary: var(--md-ref-palette-primary-20);\n`;
  css += `    --md-sys-color-primary-container: var(--md-ref-palette-primary-30);\n`;
  css += `    --md-sys-color-on-primary-container: var(--md-ref-palette-primary-90);\n`;

  css += `    --md-sys-color-secondary: var(--md-ref-palette-secondary-80);\n`;
  css += `    --md-sys-color-on-secondary: var(--md-ref-palette-secondary-20);\n`;
  css += `    --md-sys-color-secondary-container: var(--md-ref-palette-secondary-30);\n`;
  css += `    --md-sys-color-on-secondary-container: var(--md-ref-palette-secondary-90);\n`;

  css += `    --md-sys-color-surface: var(--md-ref-palette-neutral-6);\n`;
  css += `    --md-sys-color-on-surface: var(--md-ref-palette-neutral-90);\n`;
  css += `    --md-sys-color-surface-container: var(--md-ref-palette-neutral-12);\n`;
  css += `    --md-sys-color-surface-container-highest: var(--md-ref-palette-neutral-22);\n`;
  css += `    --md-sys-color-outline: var(--md-ref-palette-neutral-variant-60);\n`;
  css += `    --md-sys-color-outline-variant: var(--md-ref-palette-neutral-variant-30);\n`;
  css += `  }\n}\n`;

  return {
    seed: seedHex,
    palettes: {primary, secondary, tertiary, neutral, neutralVariant, error},
    cssCustomProperties: css,
  };
}
