/**
 * Adaptive layout hooks for React — Material Design 3 window size classes.
 *
 * The **breakpoint values and classification logic are platform-agnostic** and
 * live in `@aphrody-code/m3-tokens/breakpoints` (pure TypeScript, no DOM). This
 * module re-exports them and adds the *web* reactivity layer (`matchMedia` /
 * `resize`). That split is what makes the design system portable: a React
 * Native shell can share the very same `BREAKPOINTS` / `classifyWidth` from
 * `m3-tokens` and feed them `useWindowDimensions().width`, while a Tauri / web
 * app uses the hooks below unchanged.
 *
 * SSR-safe: during server render (and the first client render, to avoid a
 * hydration mismatch) the hooks return `ssrSizeClass` (default `"compact"`,
 * mobile-first), then settle to the real value after mount.
 */
import { useEffect, useState } from "react";
import {
  BREAKPOINTS,
  classifyWidth,
  mediaQuery,
  type WindowSizeClass,
  WINDOW_SIZE_CLASSES,
} from "@aphrody-code/m3-tokens/breakpoints";

// Re-export the canonical, platform-agnostic primitives so consumers can import
// everything adaptive from a single entrypoint (`@aphrody-code/m3-react/adaptive`).
export { BREAKPOINTS, classifyWidth, WINDOW_SIZE_CLASSES };
export type { WindowSizeClass };

/**
 * `(min-width: …)` media query string for the lower bound of a size class.
 * Alias of `m3-tokens`' `mediaQuery`, kept for the web-oriented hook API.
 */
export function minWidthQuery(sizeClass: WindowSizeClass): string {
  return mediaQuery(sizeClass);
}

const isClient = typeof window !== "undefined" && typeof window.matchMedia === "function";

/**
 * Reactive boolean for a CSS media query. SSR-safe (returns `false` until
 * mounted on the client).
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false);
  useEffect(() => {
    if (!isClient) return;
    const mql = window.matchMedia(query);
    const onChange = () => setMatches(mql.matches);
    onChange();
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [query]);
  return matches;
}

/**
 * Reactive M3 window size class for the viewport. Updates on resize/orientation
 * change via `matchMedia`. SSR-safe — returns `ssrSizeClass` until mounted.
 *
 * ```tsx
 * const sizeClass = useWindowSizeClass();
 * return sizeClass === "compact" ? <MdNavigationBar/> : <MdNavigationRail/>;
 * ```
 */
export function useWindowSizeClass(ssrSizeClass: WindowSizeClass = "compact"): WindowSizeClass {
  const [sizeClass, setSizeClass] = useState<WindowSizeClass>(ssrSizeClass);
  useEffect(() => {
    if (!isClient) return;
    const update = () => setSizeClass(classifyWidth(window.innerWidth));
    update();
    window.addEventListener("resize", update, { passive: true });
    window.addEventListener("orientationchange", update);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("orientationchange", update);
    };
  }, []);
  return sizeClass;
}

/** True when the viewport is at least the given size class (e.g. tablet+). */
export function useAtLeast(sizeClass: WindowSizeClass): boolean {
  return useMediaQuery(minWidthQuery(sizeClass));
}
