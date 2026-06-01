import { describe, expect, test } from "bun:test";
import { muiThemeToTokens, normalizeHex } from "../src/theme-to-tokens.js";

describe("normalizeHex", () => {
  test("expands #rgb, lowercases, rejects junk", () => {
    expect(normalizeHex("#FFF")).toBe("#ffffff");
    expect(normalizeHex("6750A4")).toBe("#6750a4");
    expect(normalizeHex("#6750A4")).toBe("#6750a4");
    expect(normalizeHex("rgb(0,0,0)")).toBeNull();
    expect(normalizeHex(42)).toBeNull();
  });
});

describe("muiThemeToTokens", () => {
  const theme = {
    palette: {
      mode: "light" as const,
      primary: { main: "#1976d2", contrastText: "#ffffff" },
      secondary: { main: "#9c27b0" },
      error: { main: "#d32f2f" },
      background: { default: "#fafafa", paper: "#ffffff" },
      text: { primary: "#212121", secondary: "#757575" },
      divider: "#e0e0e0",
    },
    typography: {
      fontFamily: '"Roboto", sans-serif',
      h1: { fontSize: 96, fontWeight: 300, lineHeight: 1.167 },
      body1: { fontSize: 16, lineHeight: 1.5 },
    },
    shape: { borderRadius: 8 },
  };

  test("emits :root light + prefers-color-scheme dark blocks", async () => {
    const { css } = await muiThemeToTokens(theme);
    expect(css).toContain(":root {");
    expect(css).toContain("@media (prefers-color-scheme: dark)");
    expect(css).toContain("--md-sys-color-primary:");
  });

  test("MCU is available (patched) and derives container/tertiary roles", async () => {
    const { mcuAvailable, lightRoles } = await muiThemeToTokens(theme);
    expect(mcuAvailable).toBe(true);
    // Roles MUI has no equivalent for, invented by MCU:
    expect(lightRoles["tertiary"]).toMatch(/^#[0-9a-f]{6}$/);
    expect(lightRoles["primary-container"]).toMatch(/^#[0-9a-f]{6}$/);
    expect(lightRoles["surface-variant"]).toMatch(/^#[0-9a-f]{6}$/);
  });

  test("honours the explicit MUI primary and contrastText", async () => {
    const { lightRoles } = await muiThemeToTokens(theme);
    expect(lightRoles["primary"]).toBe("#1976d2");
    expect(lightRoles["on-primary"]).toBe("#ffffff");
  });

  test("maps MUI typography variants to M3 typescale", async () => {
    const { typescale } = await muiThemeToTokens(theme);
    expect(typescale["display-large-size"]).toBe("96px");
    expect(typescale["body-large-size"]).toBe("16px");
    expect(typescale["display-large-font"]).toContain("Roboto");
  });

  test("rescales the whole M3 corner family by borderRadius / 4", async () => {
    const { shape } = await muiThemeToTokens(theme);
    // borderRadius 8 -> ratio 2 -> medium (base 12) = 24px, small (8) = 16px.
    expect(shape["corner-medium"]).toBe("24px");
    expect(shape["corner-small"]).toBe("16px");
    expect(shape["corner-full"]).toBeUndefined();
  });

  test("uses a separate dark theme when provided", async () => {
    const dark = { palette: { mode: "dark" as const, primary: { main: "#90caf9" } } };
    const { darkRoles } = await muiThemeToTokens(theme, { darkTheme: dark });
    expect(darkRoles["primary"]).toBe("#90caf9");
  });
});
