// AVANT — thème MUI (Material 2), styling Emotion.
// Source de vérité du thème : `createTheme`. Voir contrat §5 pour le mapping
// M2 → tokens M3 réalisé dans `../after/theme.css`.
import { createTheme } from "@mui/material/styles";

/**
 * Thème applicatif. Palette M2 + overrides de forme/typo.
 * Le mode (light/dark) est piloté par un état React qui re-crée le thème.
 */
export const makeTheme = (mode: "light" | "dark") =>
  createTheme({
    palette: {
      mode,
      primary: { main: "#6750A4", contrastText: "#FFFFFF" }, // → --md-sys-color-primary / on-primary
      secondary: { main: "#625B71" }, // → --md-sys-color-secondary
      error: { main: "#B3261E" }, // → --md-sys-color-error
      background: {
        default: mode === "light" ? "#FEF7FF" : "#141218", // → --md-sys-color-background/surface
      },
      divider: mode === "light" ? "#CAC4D0" : "#49454F", // → --md-sys-color-outline-variant
    },
    shape: { borderRadius: 12 }, // → --md-sys-shape-corner-*
    typography: {
      fontFamily: "Roboto, system-ui, sans-serif", // → --md-sys-typescale-*
    },
  });
