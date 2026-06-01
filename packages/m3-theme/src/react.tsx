// SPDX-License-Identifier: Apache-2.0

import React, { createContext, useContext, useState, useEffect } from "react";
import { applyDynamicFusionTheme } from "./index.js";

export type ThemeMode = "light" | "dark" | "system";
export type SchemeVariant =
  | "tonalSpot"
  | "content"
  | "fidelity"
  | "expressive"
  | "vibrant"
  | "neutral"
  | "monochrome";

export interface M3ThemeContextType {
  themeMode: ThemeMode;
  seedColor: string;
  resolvedTheme: "light" | "dark";
  contrastLevel: number;
  variant: SchemeVariant;
  setThemeMode: (mode: ThemeMode) => void;
  setSeedColor: (seed: string) => void;
  setContrastLevel: (level: number) => void;
  setVariant: (variant: SchemeVariant) => void;
}

const M3ThemeContext = createContext<M3ThemeContextType | undefined>(undefined);

export interface M3ThemeProviderProps {
  children: React.ReactNode;
  defaultSeedColor?: string;
  defaultThemeMode?: ThemeMode;
  defaultContrastLevel?: number;
  defaultVariant?: SchemeVariant;
  /** Custom element to apply the theme custom properties to. Defaults to document.documentElement. */
  target?: HTMLElement;
}

export function M3ThemeProvider({
  children,
  defaultSeedColor = "#6750a4",
  defaultThemeMode = "system",
  defaultContrastLevel = 0,
  defaultVariant = "tonalSpot",
  target,
}: M3ThemeProviderProps) {
  const [themeMode, setThemeMode] = useState<ThemeMode>(defaultThemeMode);
  const [seedColor, setSeedColor] = useState<string>(defaultSeedColor);
  const [contrastLevel, setContrastLevel] = useState<number>(defaultContrastLevel);
  const [variant, setVariant] = useState<SchemeVariant>(defaultVariant);

  // Compute resolved theme (always light or dark)
  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">(() => {
    if (typeof window === "undefined") return "light";
    if (themeMode === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return themeMode;
  });

  // Keep resolved theme in sync with themeMode and system preference changes
  useEffect(() => {
    if (themeMode !== "system") {
      setResolvedTheme(themeMode);
      return;
    }

    if (typeof window === "undefined") return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = (e: MediaQueryListEvent) => {
      setResolvedTheme(e.matches ? "dark" : "light");
    };

    setResolvedTheme(mediaQuery.matches ? "dark" : "light");
    mediaQuery.addEventListener("change", handleChange);
    return () => {
      mediaQuery.removeEventListener("change", handleChange);
    };
  }, [themeMode]);

  // Synchronize CSS custom properties on target element
  useEffect(() => {
    if (typeof window === "undefined") return;

    applyDynamicFusionTheme(seedColor, {
      dark: resolvedTheme === "dark",
      target: target ?? document.documentElement,
      contrastLevel,
      variant,
    });
  }, [seedColor, resolvedTheme, target, contrastLevel, variant]);

  const value: M3ThemeContextType = {
    themeMode,
    seedColor,
    resolvedTheme,
    contrastLevel,
    variant,
    setThemeMode,
    setSeedColor,
    setContrastLevel,
    setVariant,
  };

  return <M3ThemeContext.Provider value={value}>{children}</M3ThemeContext.Provider>;
}

export function useM3Theme(): M3ThemeContextType {
  const context = useContext(M3ThemeContext);
  if (context === undefined) {
    throw new Error("useM3Theme must be used within an M3ThemeProvider");
  }
  return context;
}
