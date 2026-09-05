// -----------------------------------------------------------------------------
// @aphrody/m3-tokens — CLI / demo (bun)
// -----------------------------------------------------------------------------
// Usage:
//   bun run demo                 # emit tokens from a realistic MUI theme
//   bun run demo '#6750A4'       # emit tokens from a Material You seed colour
//
// Prints the generated CSS to stdout; diagnostics go to stderr.
// -----------------------------------------------------------------------------

import { muiThemeToTokens, normalizeHex, type MuiTheme } from "./theme-to-tokens.js";

async function main(): Promise<void> {
  const seedArg = normalizeHex(process.argv[2] ?? "");

  const muiTheme: MuiTheme = {
    palette: {
      mode: "light",
      primary: { main: seedArg || "#1976d2", contrastText: "#ffffff" },
      secondary: { main: "#9c27b0" },
      error: { main: "#d32f2f" },
      background: { default: "#fafafa", paper: "#ffffff" },
      text: { primary: "#212121", secondary: "#757575" },
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

  const muiDark: MuiTheme = {
    palette: {
      mode: "dark",
      primary: { main: seedArg || "#90caf9" },
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
  process.stderr.write(`\n[theme-to-tokens] MCU available: ${mcuAvailable}\n\n`);
  process.stdout.write(css + "\n");
}

void main();
