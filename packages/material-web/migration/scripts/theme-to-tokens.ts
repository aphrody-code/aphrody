#!/usr/bin/env bun
// theme-to-tokens.ts
// -----------------------------------------------------------------------------
// Convertit un objet thème MUI (Material 2, `createTheme`) en un bloc CSS de
// tokens M3 `--md-sys-*` consommables par material-web (`<md-*>`).
//
// Voir le contrat : migration/00-CONVENTIONS.md (§5) et la doc :
//   docs/02-tokens-theming-web.md  +  migration/02-theme-token-migration.md
//
// Exécution :  bun migration/scripts/theme-to-tokens.ts
// Node ESM pur — pas de dépendance de build. La SEULE dépendance optionnelle est
// @material/material-color-utilities (MCU) pour dériver les rôles M3 absents de
// MUI (tertiary, *-container, surface-variant, outline…) à partir d'une couleur
// source. Si MCU est introuvable, on bascule sur un fallback de mapping direct
// (moins fidèle, documenté).
//
// IMPORTANT — pertes de fidélité M2 -> M3 (détaillées dans le .md) :
//  * MUI (M2) n'a PAS les rôles "container", "tertiary", "surface-variant",
//    "outline", "inverse-*", les surfaces tonales. MCU les *invente* depuis une
//    seule couleur source -> ce ne sont pas les couleurs choisies par le designer.
//  * MUI `primary.main` != tone M3 "primary". MCU recale la teinte sur une
//    TonalPalette : la couleur émise peut légèrement différer de `primary.main`.
//  * MUI gère light/dark via 2 thèmes distincts ; M3 dérive les deux d'une même
//    source. On respecte la source de chaque mode si fournie.
// -----------------------------------------------------------------------------

import { pathToFileURL } from "node:url";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const __dirname = import.meta.dir;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Roles = Record<string, string>;
type Tokens = Record<string, string>;

interface MuiPalette {
  mode?: string;
  primary?: { main?: string; contrastText?: string };
  secondary?: { main?: string; contrastText?: string };
  tertiary?: { main?: string };
  error?: { main?: string; contrastText?: string };
  background?: { default?: string; paper?: string };
  text?: { primary?: string; secondary?: string };
  divider?: string;
  _keepOn?: boolean;
  [k: string]: unknown;
}

interface MuiTheme {
  palette?: MuiPalette;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  typography?: Record<string, any>;
  shape?: { borderRadius?: number | string };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  colorSchemes?: { dark?: any };
}

// ---------------------------------------------------------------------------
// 1. Chargement résilient de material-color-utilities (MCU)
// ---------------------------------------------------------------------------
// On tente, dans l'ordre :
//   (a) résolution standard du package (si `bun add @material/material-color-utilities`)
//   (b) le copy déjà présent dans material-web/node_modules (repo local)
// Si rien ne marche -> mcu = null -> fallback de mapping direct.
//
// Install recommandée si absent :
//     bun add @material/material-color-utilities
// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function loadMcu(): Promise<any> {
  // (a) résolution package classique
  try {
    return await import("@material/material-color-utilities");
  } catch {
    /* on tente le repo local */
  }
  // (b) copy du repo local material-web
  const localCandidates = [
    resolve(
      __dirname,
      "../../material-web/node_modules/@material/material-color-utilities/index.js",
    ),
    resolve(__dirname, "../../node_modules/@material/material-color-utilities/index.js"),
  ];
  for (const p of localCandidates) {
    if (existsSync(p)) {
      try {
        return await import(pathToFileURL(p).href);
      } catch {
        /* continue */
      }
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// 2. Tables de correspondance
// ---------------------------------------------------------------------------

// Rôles renvoyés par Scheme.toJSON() de MCU (camelCase) -> token CSS M3 (kebab).
// Ce sont les 29 rôles du scheme "classique" MCU v0.2.x (cf. probe runtime).
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

// Mapping MUI typography.variant -> token typescale M3 <scale>-<size>.
// MUI a 13 variants ; M3 en a 15. Les variants MUI les plus proches sont alignés.
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

// Valeurs de coin M3 (cf. material-web/tokens/versions/v0_192/_md-sys-shape.scss).
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
// 3. Helpers couleur (fallback sans MCU)
// ---------------------------------------------------------------------------

// Normalise une couleur CSS hex (#rgb / #rrggbb) en #rrggbb. Renvoie null sinon.
function normalizeHex(input: unknown): string | null {
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

// Luminance relative approx -> choisit noir/blanc pour le texte "on-".
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
// 4. Génération du scheme M3 (color roles) pour un mode (light|dark)
// ---------------------------------------------------------------------------
// Retourne un objet { 'token-name': '#hex', ... } prêt à émettre en CSS.
//
// Stratégie :
//   - si MCU dispo : themeFromSourceColor(primary.main) puis on prend le scheme
//     du mode. On OVERRIDE ensuite avec les couleurs MUI réellement fournies
//     (primary/secondary/error/background/text) pour rester fidèle au design.
//   - sinon : fallback direct minimal (primary/secondary/error + on-*),
//     les rôles M3 manquants restent absents (le composant md retombe sur ses
//     fallbacks internes).
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function buildScheme(mcu: any, palette: MuiPalette, mode: string): Roles {
  const sourceHex = normalizeHex(palette?.primary?.main) || "#6750a4"; // défaut M3 (violet baseline)
  const roles: Roles = {};

  if (mcu) {
    const { themeFromSourceColor, argbFromHex, hexFromArgb } = mcu;
    const theme = themeFromSourceColor(argbFromHex(sourceHex));
    const scheme = mode === "dark" ? theme.schemes.dark : theme.schemes.light;
    const json = scheme.toJSON();
    for (const [mcuKey, token] of Object.entries(MCU_ROLE_TO_TOKEN)) {
      if (json[mcuKey] != null) roles[token] = hexFromArgb(json[mcuKey]);
    }
  } else {
    // Fallback direct : on ne dérive PAS les containers/tertiary -> on émet le
    // strict nécessaire et on laisse les composants md utiliser leurs fallbacks.
    roles.primary = sourceHex;
    roles["on-primary"] = onColorFor(sourceHex);
  }

  // --- Override par les couleurs MUI explicitement fournies (fidélité) ---
  const ov = (token: string, val: unknown, onToken?: string) => {
    const n = normalizeHex(val);
    if (n) {
      roles[token] = n;
      if (onToken && !palette?._keepOn) roles[onToken] = onColorFor(n);
    }
  };
  ov("primary", palette?.primary?.main, "on-primary");
  ov("secondary", palette?.secondary?.main, "on-secondary");
  ov("tertiary", palette?.tertiary?.main, "on-tertiary"); // si l'app MUI a un custom
  ov("error", palette?.error?.main, "on-error");

  // contrastText explicite de MUI : on respecte le choix du designer.
  const normPrimaryContrast = normalizeHex(palette?.primary?.contrastText);
  if (normPrimaryContrast) roles["on-primary"] = normPrimaryContrast;
  const normSecondaryContrast = normalizeHex(palette?.secondary?.contrastText);
  if (normSecondaryContrast) roles["on-secondary"] = normSecondaryContrast;
  const normErrorContrast = normalizeHex(palette?.error?.contrastText);
  if (normErrorContrast) roles["on-error"] = normErrorContrast;

  // Surface / background / texte / divider.
  const bg = normalizeHex(palette?.background?.default);
  const paper = normalizeHex(palette?.background?.paper);
  if (bg) {
    roles.background = bg;
    roles.surface = bg;
  }
  if (paper) roles["surface-container"] = paper; // proche : surface élevée MUI
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
// 5. Génération des tokens typescale & shape
// ---------------------------------------------------------------------------

// MUI typography -> --md-sys-typescale-*.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function buildTypescale(typography: Record<string, any> | undefined): Tokens {
  if (!typography) return {};
  const out: Tokens = {};
  // Famille de police globale -> appliquée à chaque scale (M3 n'a pas de "font
  // family" globale unique : c'est par token). On propage fontFamily à tous.
  const globalFamily: string | undefined = typography.fontFamily;

  for (const [variant, token] of Object.entries(MUI_VARIANT_TO_TYPESCALE)) {
    const v = typography[variant];
    const family: string | undefined = v?.fontFamily || globalFamily;
    if (family) out[`${token}-font`] = family;
    if (v?.fontSize != null) out[`${token}-size`] = cssLen(v.fontSize);
    if (v?.lineHeight != null)
      out[`${token}-line-height`] = cssLineHeight(v.lineHeight, v.fontSize);
    if (v?.fontWeight != null) out[`${token}-weight`] = String(v.fontWeight);
    if (v?.letterSpacing != null) out[`${token}-tracking`] = cssLen(v.letterSpacing);
  }
  return out;
}

// MUI accepte number (px) ou string. M3 attend une longueur CSS.
function cssLen(x: number | string): string {
  if (typeof x === "number") return `${x}px`;
  return String(x);
}
// MUI lineHeight peut être un ratio (1.5) -> M3 attend une longueur. On convertit
// si on connaît la fontSize numérique, sinon on émet `em`.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function cssLineHeight(lh: any, fontSize: any): string {
  if (typeof lh === "number" && lh < 4) {
    if (typeof fontSize === "number") return `${(lh * fontSize).toFixed(2)}px`;
    return `${lh}em`;
  }
  return cssLen(lh);
}

// MUI shape.borderRadius (number px) -> ré-échelonne l'échelle de coins M3.
// Subtilité : MUI `shape.borderRadius` est une *unité de base* (défaut = 4px),
// PAS l'équivalent du coin "medium" M3 (qui vaut 12px). Mapper 4 -> medium
// écraserait toute l'échelle (medium=4, small=3, large=5).
// On calcule donc un ratio = borderRadius / 4 (4 = défaut MUI) et on l'applique
// à TOUTE la famille M3 (none/extra-small/small/medium/large/extra-large), ce
// qui préserve les proportions M3 tout en respectant l'intention "arrondi" MUI.
// `corner-full` reste 9999px (pilule). Ratio 1.0 -> échelle M3 nominale.
function buildShape(shape: MuiTheme["shape"]): Tokens {
  if (!shape || shape.borderRadius == null) return {};
  const r =
    typeof shape.borderRadius === "number"
      ? shape.borderRadius
      : parseFloat(shape.borderRadius as string);
  if (!isFinite(r)) return {};
  const ratio = r / 4; // 4 = MUI default borderRadius
  const out: Tokens = {};
  for (const [corner, base] of Object.entries(M3_SHAPE_CORNERS)) {
    if (corner === "corner-full") continue; // pilule : on ne touche pas
    out[corner] = `${Math.round(base * ratio)}px`;
  }
  return out;
}

// ---------------------------------------------------------------------------
// 6. Émission CSS
// ---------------------------------------------------------------------------

function emitColorBlock(selector: string, roles: Roles): string {
  const lines = Object.entries(roles).map(([t, v]) => `  --md-sys-color-${t}: ${v};`);
  return `${selector} {\n${lines.join("\n")}\n}`;
}

function emitMiscBlock(selector: string, typescale: Tokens, shape: Tokens): string {
  const lines = [
    ...Object.entries(typescale).map(([t, v]) => `  --md-sys-typescale-${t}: ${v};`),
    ...Object.entries(shape).map(([t, v]) => `  --md-sys-shape-${t}: ${v};`),
  ];
  if (lines.length === 0) return "";
  return `${selector} {\n${lines.join("\n")}\n}`;
}

// ---------------------------------------------------------------------------
// 7. API principale : muiThemeToTokens(theme, { mode, darkTheme })
// ---------------------------------------------------------------------------
// theme : objet thème MUI (palette/typography/shape). Mode déterminé par
//         theme.palette.mode (ou option). Si un thème dark séparé est fourni,
//         il alimente le bloc @media (prefers-color-scheme: dark).
export async function muiThemeToTokens(
  lightTheme: MuiTheme,
  options: { mode?: string; darkTheme?: MuiTheme | MuiPalette } = {},
): Promise<{
  css: string;
  mcuAvailable: boolean;
  lightRoles: Roles;
  darkRoles: Roles;
  typescale: Tokens;
  shape: Tokens;
}> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mcu: any = await loadMcu();

  const lightPalette: MuiPalette = { ...lightTheme.palette, mode: "light" };
  const lightRoles = buildScheme(mcu, lightPalette, "light");

  // typescale + shape sont indépendants du mode -> émis une fois dans :root.
  const typescale = buildTypescale(lightTheme.typography);
  const shape = buildShape(lightTheme.shape);

  // Bloc dark : soit un thème dark fourni explicitement (colorSchemes.dark de
  // MUI v6+ ou un createTheme({palette:{mode:'dark'}})), soit dérivé par MCU.
  const darkTheme = options.darkTheme || lightTheme.colorSchemes?.dark || null;
  const darkPalette: MuiPalette = darkTheme
    ? { ...((darkTheme as MuiTheme).palette || darkTheme), mode: "dark" }
    : { ...lightPalette, mode: "dark" };
  const darkRoles = buildScheme(mcu, darkPalette, "dark");

  const parts: string[] = [];
  parts.push(`/* Généré par migration/scripts/theme-to-tokens.ts */`);
  parts.push(
    `/* MCU: ${mcu ? `disponible (${mcu.themeFromSourceColor ? "API ok" : "?"})` : "ABSENT -> fallback direct (fidélité réduite)"} */`,
  );
  parts.push(`:root {\n  color-scheme: light dark;\n}`);
  const misc = emitMiscBlock(":root", typescale, shape);
  parts.push(emitColorBlock(":root", lightRoles));
  if (misc) parts.push(misc);
  parts.push(
    `@media (prefers-color-scheme: dark) {\n${indent(emitColorBlock(":root", darkRoles))}\n}`,
  );

  return {
    css: parts.join("\n\n"),
    mcuAvailable: !!mcu,
    lightRoles,
    darkRoles,
    typescale,
    shape,
  };
}

function indent(block: string): string {
  return block
    .split("\n")
    .map((l) => "  " + l)
    .join("\n");
}

// ---------------------------------------------------------------------------
// 8. Démo / test exécutable
// ---------------------------------------------------------------------------
if (import.meta.main) {
  // Thème MUI réaliste (équivalent du défaut MUI : primary bleu, secondary rose).
  const muiTheme: MuiTheme = {
    palette: {
      mode: "light",
      primary: { main: "#1976d2", contrastText: "#ffffff" },
      secondary: { main: "#9c27b0" },
      error: { main: "#d32f2f" },
      background: { default: "#fafafa", paper: "#ffffff" },
      text: {
        primary: "rgba(0,0,0,0.87)".replace("rgba(0,0,0,0.87)", "#212121"),
        secondary: "#757575",
      },
      divider: "#e0e0e0",
    },
    typography: {
      fontFamily: '"Roboto", "Helvetica", "Arial", sans-serif',
      h1: {
        fontSize: 96,
        fontWeight: 300,
        lineHeight: 1.167,
        letterSpacing: -1.5,
      },
      h6: { fontSize: 20, fontWeight: 500, lineHeight: 1.6 },
      body1: { fontSize: 16, fontWeight: 400, lineHeight: 1.5 },
      button: { fontSize: 14, fontWeight: 500, lineHeight: 1.75 },
    },
    shape: { borderRadius: 4 },
  };

  // Thème dark séparé (façon MUI : un 2e createTheme, ici juste le palette).
  const muiDark: MuiTheme = {
    palette: {
      mode: "dark",
      primary: { main: "#90caf9" },
      secondary: { main: "#ce93d8" },
      error: { main: "#f44336" },
      background: { default: "#121212", paper: "#1e1e1e" },
      text: { primary: "#ffffff", secondary: "#b0b0b0" },
      divider: "#373737",
    },
  };

  const { css, mcuAvailable } = await muiThemeToTokens(muiTheme, {
    darkTheme: muiDark,
  });
  console.error(`\n[theme-to-tokens] MCU disponible : ${mcuAvailable}\n`);
  console.log(css);
}
