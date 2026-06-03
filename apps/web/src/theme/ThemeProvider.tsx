// Applies Material You dynamic color from the active seed + mode onto <html>,
// re-running whenever the seed, the theme mode, or the OS preference changes.

import { useEffect } from "react";
import { applyDynamicColor } from "@aphrody/m3-tokens/dynamic-color";
import { isDark, useUi } from "../store.ts";

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { seed, themeMode } = useUi();

  useEffect(() => {
    const apply = () => {
      const dark = isDark(themeMode);
      applyDynamicColor(seed, { dark });
      document.documentElement.dataset.theme = dark ? "dark" : "light";
      document.documentElement.style.colorScheme = dark ? "dark" : "light";
    };
    apply();

    if (themeMode !== "system") return;
    const mq = matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [seed, themeMode]);

  return children;
}
